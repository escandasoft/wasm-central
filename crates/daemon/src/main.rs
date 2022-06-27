use std::borrow::BorrowMut;
use std::ops::{Deref, DerefMut};
use std::ptr::null_mut;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wasm_central_runner::modules::ModuleManager;

use std::vec::Vec;

use clap::Parser;
use console::Style;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use std::thread;

use crate::cli_proto::modules_server::ModulesServer;
use crate::cli_proto::modules_server::Modules;
use crate::cli_proto::*;

#[derive(Parser)]
struct Cli {
    /// Host addr interface to listen to
    address: String,
    /// Port to listen to
    port: u16,
    modules_path: std::path::PathBuf,
}

pub mod cli_proto {
    tonic::include_proto!("cli_proto");
}

pub struct MyModules {
    mutex: Mutex<u8>,
    manager: Arc<Mutex<ModuleManager>>,
}

impl MyModules {
    pub fn new(manager: Arc<Mutex<ModuleManager>>) -> MyModules {
        MyModules {
            mutex: std::sync::Mutex::new(0),
            manager,
        }
    }
}

#[tonic::async_trait]
impl Modules for MyModules {
    async fn list(&self, request: Request<ModuleListRequest>) -> Result<Response<ModuleListReply>, Status> {
        return match self.mutex.lock() {
            Ok(_lock) => {
                let items = self
                    .manager
                    .lock()
                    .unwrap()
                    .running_modules()
                    .iter()
                    .map(|loaded_module| {
                        let module_status = loaded_module.status;
                        ModuleListReplyItem {
                            name: String::from(&loaded_module.name),
                            status: module_status.as_string(),
                            successes: 0,
                            failures: 0,
                            total_messages: 0,
                            fail_rate_per_minute: 0.0,
                        }
                    })
                    .collect::<Vec<ModuleListReplyItem>>();
                Ok(Response::new(ModuleListReply {
                    items: items.clone(),
                    item_no: items.len() as i32,
                }))
            }
            _ => {
                Err(Status::new(tonic::Code::from(500), "cannot acquire lock"))
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

const MODULE_MANAGER_LOOP_WAIT: u64 = 1000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();

    let path = args.modules_path;
    let addr = args.address;
    let port = args.port;
    let maddr = format!("{}:{}", addr, port);
    let faddr = maddr.parse().unwrap();

    let blue = Style::new().blue();

    let mut mgr = Arc::new(Mutex::new(ModuleManager::new(path.clone())));

    let modules_server = ModulesServer::new(MyModules::new(mgr.clone()));
    let bootstrap_future = Server::builder().add_service(modules_server).serve(faddr);
    println!("Server ready at {}", blue.apply_to(faddr));

    let mut mgr = Arc::clone(&mgr);
    thread::spawn(move || {
        loop {
            mgr.lock().unwrap().tick();
            thread::sleep(Duration::from_millis(MODULE_MANAGER_LOOP_WAIT));
        }
    });
    bootstrap_future.await?;
    return Ok(());
}
