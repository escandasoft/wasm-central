use std::sync::atomic::{AtomicBool, Ordering};
use std::vec::Vec;

use clap::Parser;
use console::Style;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use crate::datatx_proto::modules_server::{Modules, ModulesServer};
use crate::datatx_proto::*;

use wasm_central::watcher::DirectoryWatcher;

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

#[derive(Debug)]
pub struct LoadedModule {
    name: String,
    path: String,
    status: String,
    processes: Vec<i32>,
}

pub struct MyModules {
    hot_folder_path: std::path::PathBuf,
    mutex: std::sync::Mutex<u8>,
    loaded_modules: Vec<LoadedModule>,
    is_running: AtomicBool,
}

impl MyModules {
    fn new(path: std::path::PathBuf) -> MyModules {
        MyModules {
            hot_folder_path: path,
            is_running: AtomicBool::new(false),
            loaded_modules: vec![],
            mutex: std::sync::Mutex::new(0),
        }
    }
}

#[tonic::async_trait]
impl Modules for MyModules {
    async fn list(&self, request: Request<Empty>) -> Result<Response<ModuleListReply>, Status> {
        match self.mutex.lock() {
            Ok(lock) => {
                let items = self
                    .loaded_modules
                    .iter()
                    .map(|loaded_module| {
                        return ModuleListReplyItem {
                            name: String::from(&loaded_module.name),
                            status: loaded_module.status.clone(),
                            successes: 0,
                            failures: 0,
                            total_messages: 0,
                            fail_rate_per_minute: 0.0,
                        };
                    })
                    .collect::<Vec<ModuleListReplyItem>>();
                return Ok(Response::new(ModuleListReply {
                    items: items,
                    item_no: self.loaded_modules.len() as i32,
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

    let watcher = DirectoryWatcher::new(path.clone());
    let service = MyModules::new(path.clone());

    let bootstrap_future = Server::builder()
        .add_service(ModulesServer::new(service))
        .serve(faddr);
    println!("Server ready at {}", blue.apply_to(faddr));
    bootstrap_future.await?;
    return Ok(());
}
