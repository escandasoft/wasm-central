mod compiler;
mod options;

use std::fs;
use std::io::Read;
use std::os::unix::fs::{FileExt, MetadataExt};
use clap::Parser;
use options::{Args, ModuleCommands};

use tonic::Request;
use tokio_stream::{Stream, StreamExt};
use tonic::IntoStreamingRequest;
use tonic::Streaming;
use tokio_stream::FromStream;

use crate::cli_proto::modules_client::ModulesClient;
use crate::cli_proto::*;

pub mod cli_proto {
    tonic::include_proto!("cli_proto");
}

#[tokio::main]
#[warn(non_snake_case)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if let Some(command) = args.command {
        match command {
            ModuleCommands::List { host, port } => {
                let mut client = ModulesClient::connect(format!("http://{}:{}", host, port))
                    .await
                    .unwrap_or_else(|_| panic!("Cannot connect to server at {}:{}", host, port));
                match client.list(ModuleListRequest {}).await {
                    Ok(response) => {
                        let reply = response.into_inner();
                        println!("Found {} modules", reply.item_no);
                        println!("NAME\t\tSTATE");
                        for item in reply.items {
                            println!("{}\t\t{}", item.name, item.status)
                        }
                    }
                    Err(err) => println!("Cannot list modules: {}", err.message()),
                }
            }
            ModuleCommands::Compile {
                base,
                input_file,
                output_file,

            } => {
                compiler::compile(&base, &input_file, &output_file);
            }
            ModuleCommands::Deploy {
                host,
                port,
                file_path,
                inputs,
                outputs
            } => {
                let mut client = ModulesClient::connect(format!("http://{}:{}", host, port))
                    .await
                    .unwrap_or_else(|_| panic!("Cannot connect to server at {}:{}", host, port));
                match fs::File::open(file_path.clone()) {
                    Ok(mut file) => {
                        let iterable = tokio_stream::iter(0..(file.metadata().unwrap().size() / 1024 + 1)).map(move |i| {
                            let mut buffer = Vec::with_capacity(1024);
                            let read = file.read_at(&mut buffer[..], i * 1024 as u64)
                                .expect("Cannot read buffer");
                            ModuleLoadPartRequest {
                                file_name: file_path.clone().file_name().unwrap().to_str().unwrap().to_owned(),
                                inputs: inputs.clone(),
                                outputs: outputs.clone(),
                                zip_file_bytes: buffer[0..read].to_vec()
                            }
                        });
                        println!("Starting to stream file to server");
                        match client.load(iterable).await {
                            Ok(response) => {
                                let response = response.into_inner();
                                if !response.success {
                                    eprintln!("Cannot deploy because {}", response.error_message.unwrap_or("unknown".to_owned()));
                                } else {
                                    println!("Successfully deployed to server");
                                }
                            },
                            Err(err) => {
                                eprintln!("Cannot deploy because error {:?}", err);
                            }
                        };
                    }
                    Err(_) => {}
                }
            }
        }
    }
    Ok(())
}
