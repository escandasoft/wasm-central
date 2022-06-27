use std::borrow::BorrowMut;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::ptr::null_mut;
use std::rc::Rc;
use std::iter::{Iterator, zip};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wasm_central_runner::modules::ModuleManager;

use std::vec::Vec;

use clap::Parser;
use console::Style;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use std::{fs, thread};
use std::os::unix::fs::FileExt;

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
    manager: Arc<Mutex<ModuleManager>>,
}

impl MyModules {
    pub fn new(manager: Arc<Mutex<ModuleManager>>) -> MyModules {
        MyModules {
            manager,
        }
    }
}

#[tonic::async_trait]
impl Modules for MyModules {
    async fn list(&self, request: Request<ModuleListRequest>) -> Result<Response<ModuleListReply>, Status> {
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

    async fn load(
        &self,
        request: Request<Streaming<ModuleLoadPartRequest>>,
    ) -> Result<Response<ModuleLoadReply>, Status> {
        let mut streaming = request.into_inner();
        let mut success = false;
        let mut full_path = None;
        let mut offset = 0 as u64;
        while let Some(item) = streaming.message().await? {
            let path = full_path.unwrap_or_else(|| {
                let rel_file_path = PathBuf::from(item.file_name);
                self.manager.lock().unwrap().watcher.dir.join(rel_file_path)
            });
            full_path = Some(path.clone());
            let zip_file_bytes = item.zip_file_bytes;
            let open_file = move || {
                let the_path = path.clone();
                if the_path.exists() {
                    fs::File::open(the_path.clone())
                } else {
                    fs::File::create(the_path.clone())
                }
            };
            match open_file() {
                Ok(mut file) => {
                    let old_offset = offset;
                    offset += zip_file_bytes.len() as u64;
                    file.write_at(&zip_file_bytes[..], old_offset)
                }
                Err(err) => {
                    success = false;
                    eprintln!("Cannot open file for writing {:?}", err);
                    Err(err)
                }
            }?;
        }
        let error_message = if !success {
            Some(format!("Cannot load file at {}", full_path.unwrap().display()))
        } else {
            None
        };
        let reply = ModuleLoadReply {
            success: !success,
            error_message,
            time: 0,
        };
        Ok(Response::new(reply))
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

    let mgr = Arc::new(Mutex::new(ModuleManager::new(path.clone())));

    let modules_server = ModulesServer::new(MyModules::new(mgr.clone()));
    let bootstrap_future = Server::builder().add_service(modules_server).serve(faddr);
    println!("Server ready at {}", blue.apply_to(faddr));

    let mgr = Arc::clone(&mgr);
    thread::spawn(move || {
        loop {
            mgr.lock().unwrap().tick();
            thread::sleep(Duration::from_millis(MODULE_MANAGER_LOOP_WAIT));
        }
    });
    bootstrap_future.await?;
    return Ok(());
}
