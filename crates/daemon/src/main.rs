use std::iter::Iterator;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wasm_central_runner::functions::{FunctionManager, FunctionStatus};

use std::vec::Vec;

use clap::Parser;
use console::Style;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use iter_tools::Itertools;
use prost::Message;
use std::io::{Read, Write};
use std::{fs, str, thread};
use zip::write::FileOptions;
use wasm_central_runner::data::DataFrame;

use crate::fn_proto::executor_server::Executor;
use crate::fn_proto::executor_server::ExecutorServer;
use crate::fn_proto::*;

use crate::mgmt_proto::manager_server::Manager;
use crate::mgmt_proto::manager_server::ManagerServer;
use crate::mgmt_proto::*;

#[derive(Parser)]
struct Cli {
    /// Host addr interface to listen to
    address: String,
    /// Port to listen to
    port: u16,
    modules_path: PathBuf,
}

pub mod fn_proto {
    tonic::include_proto!("fn_proto");
}

pub mod mgmt_proto {
    tonic::include_proto!("mgmt_proto");
}

pub struct Impl {
    manager: Arc<Mutex<FunctionManager>>,
}

impl Impl {
    pub fn new(manager: Arc<Mutex<FunctionManager>>) -> Impl {
        Impl { manager }
    }
}

#[tonic::async_trait]
impl Executor for Impl {
    async fn execute(
        &self,
        request: Request<ExecuteRequest>,
    ) -> Result<Response<ExecuteReply>, Status> {
        let req = request.into_inner();
        if let Some(handle) = self.manager.lock()
            .unwrap()
            .get_handle(&req.name) {
            match handle.run(&DataFrame {
                body: req.body.clone().into()
            }) {
                Ok(output) => {
                    println!("Executed function");
                    Ok(Response::new(ExecuteReply {
                        body: output.body
                    }))
                }
                Err(err) => {
                    eprintln!("Error executing function");
                    Err(Status::internal(format!("{:?}", err)))
                }
            }
        } else {
            Ok(Response::new(ExecuteReply {
                body: "Couldn't execute fn".to_string().as_bytes().to_vec()
            }))
        }
    }
}

#[tonic::async_trait]
impl Manager for Impl {
    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<ListReply>, Status> {
        let items = self
            .manager
            .lock()
            .unwrap()
            .running_modules()
            .iter()
            .map(|loaded_module| {
                let module_status = loaded_module.status;
                ListReplyItem {
                    name: String::from(&loaded_module.name),
                    status: module_status.as_string(),
                    successes: 0,
                    failures: 0,
                    total_messages: 0,
                    fail_rate_per_minute: 0.0,
                }
            })
            .collect::<Vec<ListReplyItem>>();
        Ok(Response::new(ListReply {
            items: items.clone(),
            item_no: items.len() as i32,
        }))
    }

    async fn load(
        &self,
        request: Request<Streaming<LoadPartRequest>>,
    ) -> Result<Response<LoadReply>, Status> {
        let mut streaming = request.into_inner();
        let success = true;
        let mut module_name = String::new();
        let rt_path = self.manager.lock().unwrap().watcher.dir.clone();
        if let Some(item) = streaming.message().await? {
            let full_path = rt_path.join(format!("{}.{}", item.name.clone(), "wasm"));
            let mut file = fs::File::create(full_path.clone())?;
            file.write_all(&item.body)?;
            println!("!! wrote {} bytes", item.body.len());
            while let Some(item) = streaming.message().await? {
                file.write_all(&item.body)?;
                println!("!! wrote {} bytes", item.body.len());
            }
            file.flush()?;
        } else {
            eprintln!("Cannot receive file stream");
        }
        let error_message = if success {
            None
        } else {
            Some("Cannot load file".to_owned())
        };
        self.manager.lock().unwrap().tick();
        let map = self.manager.lock().unwrap().running_modules_map();
        let module_status = map
            .get(module_name.as_str())
            .map(|i| i.status)
            .or_else(|| Some(FunctionStatus::Undeployed))
            .unwrap();
        let reply = LoadReply {
            success: success && module_status.eq(&FunctionStatus::Deploy),
            error_message,
            time: 0,
        };
        Ok(Response::new(reply))
    }

    async fn unload(
        &self,
        request: Request<UnloadRequest>,
    ) -> Result<Response<UnloadReply>, Status> {
        return Ok(Response::new(UnloadReply {
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

    let mgr = Arc::new(Mutex::new(FunctionManager::new(path.clone())));

    let mgmt_server = ManagerServer::new(Impl::new(mgr.clone()));
    let executor_server = ExecutorServer::new(Impl::new(mgr.clone()));
    let bootstrap_future = Server::builder()
        .add_service(mgmt_server)
        .add_service(executor_server)
        .serve(faddr);
    println!("Server ready at {}", blue.apply_to(faddr));

    let mgr = Arc::clone(&mgr);
    thread::spawn(move || loop {
        mgr.lock().unwrap().tick();
        thread::sleep(Duration::from_millis(MODULE_MANAGER_LOOP_WAIT));
    });
    bootstrap_future.await?;
    return Ok(());
}
