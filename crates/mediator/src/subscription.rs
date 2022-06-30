use clap::Parser;
use clap::Subcommand;
use kafka::Error;
use std::collections::HashMap;
use std::iter::FromFn;
use std::io::ErrorKind;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{LockResult, Mutex};
use std::time::Duration;
use kafka::consumer::Consumer;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{iter, Stream, StreamExt};
use tonic::async_trait;
use tonic::codegen::{Body, futures_core};
use tonic::{Code, IntoStreamingRequest, Response, Status};

use fn_proto::subscriber_server::Subscriber;
use fn_proto::*;

pub mod fn_proto {
    tonic::include_proto!("fn_proto");
}

pub struct MySubscriber {
    pub(crate) hosts: Vec<String>,
}

#[async_trait]
impl Subscriber for MySubscriber {
    type SubscribeStream = Pin<Box<dyn Stream<Item=Result<TopicResult, Status>> + Send>>;

    async fn subscribe(
        &self,
        request: tonic::Request<tonic::Streaming<Topic>>,
    ) -> Result<tonic::Response<Self::SubscribeStream>, tonic::Status> {
        let mut in_stream = request.into_inner();

        let (tx, rx) = mpsc::channel(128);

        let hosts = self.hosts.clone();
        tokio::spawn(async move {
            let mut builder = Consumer::from_hosts(hosts);
            if let Some(Ok(topic)) = in_stream.next().await {
                for topic in topic.topic_names {
                    builder = builder.with_topic(topic);
                }
                if let Ok(mut consumer) = builder.create() {
                    while let Ok(message_sets) = consumer.poll() {
                        let mut messages = vec![];
                        for mss in message_sets.iter() {
                            for m in mss.messages() {
                                messages.push(TopicMessage {
                                    topic: mss.topic().to_string(),
                                    key: m.key.to_vec(),
                                    value: m.value.to_vec(),
                                });
                            }
                            if let Err(err) = consumer.consume_messageset(mss) {
                                eprintln!("Cannot consume message set");
                                return Err(Status::new(Code::Internal, "Cannot consume message set"));
                            }
                        }
                        tx.send(Ok(TopicResult { messages }))
                            .await
                            .map_err(|err| Status::internal("Cannot send to remote thread item"))?;
                    }
                }
            }
            Ok(())
        });

        // echo just write the same data that was received
        let out_stream = ReceiverStream::new(rx);

        Ok(Response::new(
            Box::pin(out_stream) as Self::SubscribeStream
        ))
    }
}