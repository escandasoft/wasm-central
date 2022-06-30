mod compiler;
mod options;

use clap::Parser;
use options::{Args, ModuleCommands};
use std::io::Read;
use std::{cmp, fs};

use tokio_stream::StreamExt;
use tonic::IntoStreamingRequest;

use crate::fn_proto::functions_client::FunctionsClient;
use crate::fn_proto::*;

pub mod fn_proto {
    tonic::include_proto!("fn_proto");
}

#[tokio::main]
#[warn(non_snake_case)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if let Some(command) = args.command {
        match command {
            ModuleCommands::List { host, port } => {
                let mut client = FunctionsClient::connect(format!("http://{}:{}", host, port))
                    .await
                    .unwrap_or_else(|_| panic!("Cannot connect to server at {}:{}", host, port));
                match client.list(FunctionListRequest {}).await {
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
                input_file,
                output_file,
            } => {
                compiler::compile(&input_file, &output_file);
            }
            ModuleCommands::Deploy {
                host,
                port,
                file_path,
                inputs,
                outputs,
            } => {
                let mut client = FunctionsClient::connect(format!("http://{}:{}", host, port))
                    .await
                    .unwrap_or_else(|_| panic!("Cannot connect to server at {}:{}", host, port));
                match fs::File::open(file_path.clone()) {
                    Ok(mut file) => {
                        const BUFFER_SIZE: usize = 1024 * 1024;
                        let mut buffer = vec![];
                        file.read_to_end(&mut buffer)
                            .expect("Cannot write to buffer");
                        let iterable = tokio_stream::iter(0..((buffer.len() / BUFFER_SIZE) + 1))
                            .map(move |i| {
                                let offset = i * BUFFER_SIZE;
                                println!("!! made ModuleLoadPartRequest {}", i);
                                let top = cmp::min(buffer.len(), offset + BUFFER_SIZE);
                                let range = offset..top;
                                {
                                    let fmt = range.clone();
                                    println!("!! sending range ({}, {})", fmt.start, fmt.end);
                                }
                                FunctionLoadPartRequest {
                                    file_name: file_path
                                        .clone()
                                        .file_name()
                                        .unwrap()
                                        .to_str()
                                        .unwrap()
                                        .to_owned(),
                                    inputs: inputs.clone(),
                                    outputs: outputs.clone(),
                                    runnable_bytes: buffer[range].to_vec(),
                                }
                            });
                        println!("Starting to stream file to server");
                        match client.load(iterable).await {
                            Ok(response) => {
                                let response = response.into_inner();
                                if !response.success {
                                    eprintln!(
                                        "Cannot deploy because {}",
                                        response.error_message.unwrap_or("unknown".to_owned())
                                    );
                                } else {
                                    println!("Successfully deployed to server");
                                }
                            }
                            Err(err) => {
                                eprintln!("Cannot deploy because remote error: {:?}", err);
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
