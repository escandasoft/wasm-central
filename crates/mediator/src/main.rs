mod subscription;

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
use tonic::transport::Server;
use crate::subscription::fn_proto::*;
use crate::subscription::fn_proto::functions_client::FunctionsClient;
use crate::subscription::fn_proto::subscriber_server::SubscriberServer;
use crate::subscription::fn_proto::subscriber_client::SubscriberClient;
use crate::subscription::MySubscriber;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let hosts = [format!("{}::{}", args.kafka_host, args.kafka_port)];
    Ok(())
}
