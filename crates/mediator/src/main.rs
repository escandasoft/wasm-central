use std::collections::HashMap;
use clap::Parser;
use clap::Subcommand;

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    Ok(())
}
