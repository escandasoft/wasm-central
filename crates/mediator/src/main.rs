use clap::Parser;
use clap::Subcommand;
use kafka::Error;
use std::collections::HashMap;
use std::iter::FromFn;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{LockResult, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{iter, Stream, StreamExt};
use tonic::async_trait;
use tonic::codegen::{Body, futures_core};
use tonic::{Code, IntoStreamingRequest, Response, Status};

use crate::fn_proto::subscriber_server::Subscriber;
use crate::fn_proto::*;

pub mod fn_proto {
    tonic::include_proto!("fn_proto");
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short = 'K', long = "kafka-host")]
    pub kafka_host: String,

    #[clap(short = 'k', long = "kafka-port")]
    pub kafka_port: i16,

    #[clap(short = 'S', long = "schema-host")]
    pub schema_host: String,

    #[clap(short = 's', long = "schema-port")]
    pub schema_port: i16,

    #[clap(short = 'F', long = "fn-host")]
    pub fn_host: String,

    #[clap(short = 'f', long = "fn-port")]
    pub fn_port: i16,
}

struct Subscription {
    consumer: kafka::consumer::Consumer,
}

struct MySubscriber {
    hosts: Vec<String>,
}

#[async_trait]
impl Subscriber for MySubscriber {
    type SubscribeStream = Pin<Box<dyn Stream<Item=Result<TopicResult, Status>> + Send>>;

    async fn subscribe(
        &self,
        request: tonic::Request<tonic::Streaming<Topic>>,
    ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
        let topic = request.into_inner();
        let consumer = kafka::consumer::Consumer::from_hosts(self.hosts.clone())
            .create()
            .map_err(|err| Status::new(Code::Internal, "Cannot create consumer"))?;
        let mut consumer = Mutex::new(consumer);
        let inner_stream = tokio_stream::iter(0..).map(move |_i| {
            let message_sets = consumer.lock().unwrap().poll();
            let mut messages = vec![];
            for mss in message_sets {
                for ms in mss.iter() {
                    for m in ms.messages() {
                        messages.push(TopicMessage {
                            key: m.key.to_vec(),
                            value: m.value.to_vec(),
                        });
                    }
                    if let Err(err) = consumer.lock().unwrap().consume_messageset(ms) {
                        eprintln!("Cannot consume message set");
                        return Err(Status::new(Code::Internal, "Cannot consume message set"));
                    }
                }
            }
            Ok(TopicResult { messages })
        });
        let mut stream = Box::pin(inner_stream);
        Ok(Response::new(stream as Self::SubscribeStream))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let hosts = [format!("{}::{}", args.kafka_host, args.kafka_port)];
    let subscriber = MySubscriber {
        hosts: hosts.to_vec(),
    };
    let topic = Topic {
        topic_names: ["input-topic".to_string()].to_vec(),
    };
    let stream = tokio_stream::once(topic);
    Ok(())
}
