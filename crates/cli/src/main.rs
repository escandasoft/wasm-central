mod compiler;
mod options;

use clap::Parser;
use options::{Args, ModuleCommands};

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
            } => {
                let mut client = ModulesClient::connect(format!("http://{}:{}", host, port))
                    .await
                    .unwrap_or_else(|_| panic!("Cannot connect to server at {}:{}", host, port));
                /*
                match client
                    .load(ModuleLoadPartRequest {
                        zip_file_bytes: vec![],
                    })
                    .await
                {
                    Ok(response) => {}
                }*/
            }
        }
    }
    Ok(())
}
