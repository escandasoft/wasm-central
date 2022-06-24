use wasm_central_runner::modules::ModuleManager;

use std::vec::Vec;

use clap::Parser;
use console::Style;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use crate::datatx_proto::modules_server::{Modules, ModulesServer};
use crate::datatx_proto::*;
use wasm_central_runner::modules::ModuleStatus;

#[derive(Parser)]
struct Cli {
    /// Host addr interface to listen to
    address: String,
    // Port to listen to
    port: u16,
    modules_path: std::path::PathBuf,
}

pub mod datatx_proto {
    tonic::include_proto!("datatx_proto");
}

pub struct MyModules {
    mutex: std::sync::Mutex<u8>,
    manager: ModuleManager,
}

impl MyModules {
    fn new(path: std::path::PathBuf) -> MyModules {
        MyModules {
            mutex: std::sync::Mutex::new(0),
            manager: ModuleManager::new(path.clone()),
        }
    }
}

#[tonic::async_trait]
impl Modules for MyModules {
    async fn list(&self, request: Request<Empty>) -> Result<Response<ModuleListReply>, Status> {
        match self.mutex.lock() {
            Ok(_lock) => {
                let items = self
                    .manager
                    .running_modules()
                    .iter()
                    .map(|loaded_module| {
                        let module_status = loaded_module.status.to_string();
                        ModuleListReplyItem {
                            name: String::from(&loaded_module.name),
                            status: String::from(module_status),
                            successes: 0,
                            failures: 0,
                            total_messages: 0,
                            fail_rate_per_minute: 0.0,
                        }
                    })
                    .collect::<Vec<ModuleListReplyItem>>();
                return Ok(Response::new(ModuleListReply {
                    items: items.clone(),
                    item_no: items.len() as i32,
                }));
            }
            _ => {
                return Err(Status::new(tonic::Code::from(500), "cannot acquire lock"));
            }
        }
    }

    async fn load(
        &self,
        request: Request<Streaming<ModuleLoadPartRequest>>,
    ) -> Result<Response<ModuleLoadReply>, Status> {
        return Ok(Response::new(ModuleLoadReply {
            success: false,
            error_message: None,
            time: 0,
        }));
    }

    async fn replace(
        &self,
        request: Request<ModuleReplaceRequest>,
    ) -> Result<Response<ModuleReplaceReply>, Status> {
        return Ok(Response::new(ModuleReplaceReply {
            success: false,
            error_message: None,
            time: 0,
        }));
    }

    async fn unload(
        &self,
        request: Request<ModuleUnloadRequest>,
    ) -> Result<Response<ModuleUnloadReply>, Status> {
        return Ok(Response::new(ModuleUnloadReply {
            success: false,
            error_message: None,
            unloaded_module_name: String::from("proc"),
            time: 0,
        }));
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let path = args.modules_path;
    let addr = args.address;
    let port = args.port;
    let maddr = format!("{}:{}", addr, port);
    let faddr = maddr.parse().unwrap();

    let blue = Style::new().blue();

    let service = MyModules::new(path.clone());

    let bootstrap_future = Server::builder()
        .add_service(ModulesServer::new(service))
        .serve(faddr);
    println!("Server ready at {}", blue.apply_to(faddr));
    bootstrap_future.await?;
    return Ok(());
}
