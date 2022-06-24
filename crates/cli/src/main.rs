mod compiler;
mod options;

use clap::Parser;
use options::{Args, ModuleCommands};

use crate::datatx_proto::modules_client::ModulesClient;
use crate::datatx_proto::*;

pub mod datatx_proto {
    tonic::include_proto!("datatx_proto");
}

#[tokio::main]
#[warn(non_snake_case)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut client = create_client();

    if let Some(command) = args.command {
        match command {
            ModuleCommands::List { host, port } => match client.list(Empty {}).await {
                Ok(response) => {
                    let reply = response.into_inner();
                    println!("Found {} modules", reply.item_no);
                    println!("NAME\t\tSTATE");
                    for item in reply.items {
                        println!("{}\t\t{}", item.name, item.status)
                    }
                }
                Err(err) => println!("Cannot list modules: {}", err.message()),
            },
            ModuleCommands::Compile {
                base,
                input_file,
                output_file,
            } => match compiler::compile(&base, &input_file, &output_file) {
                Ok(()) => println!("Successfully compiled JS into file {:?}", output_file),
                Err(error) => println!(),
            },
            ModuleCommands::Deploy { host, port, file_path } => {}
        }
    }
    Ok(())
}

async fn create_client() {
    ModulesClient::connect(format!(
        "http://{}:{}",
        host, port
    ))
        .await
        .unwrap_or_else(|_| {
            panic!(
                "Cannot connect to server at {}:{}",
                host, port
            )
        })
}
