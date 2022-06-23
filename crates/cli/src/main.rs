use clap::{Parser, Subcommand};

use crate::datatx_proto::modules_client::ModulesClient;
use crate::datatx_proto::*;

pub mod datatx_proto {
    tonic::include_proto!("datatx_proto");
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[clap(short, long)]
    server_address: String,

    /// Number of times to greet
    #[clap(subcommand)]
    command: Option<ModuleCommands>,
}

#[derive(Subcommand)]
enum ModuleCommands {
    List {},
    Load {
        #[clap(short, long)]
        file_path: std::path::PathBuf,
    },
    Compile {
        #[clap(short, long)]
        base: std::path::PathBuf,

        #[clap(short, long)]
        entry_file: std::path::PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let server_address = args.server_address;
    let mut client = ModulesClient::connect(server_address.clone())
        .await
        .expect(&format!(
            "Cannot connect to server at {}",
            server_address.clone()
        ));

    match args.command.unwrap() {
        List => match client.list(Empty {}).await {
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
        Compile => {
        },
        Load => {},
    }
    return Ok(());
}
