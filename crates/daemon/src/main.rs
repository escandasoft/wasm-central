use std::borrow::BorrowMut;
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::ptr::null_mut;
use std::rc::Rc;
use std::iter::{Iterator, zip};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wasm_central_runner::modules::{ModuleManager, ModuleStatus};

use std::vec::Vec;

use clap::Parser;
use console::Style;
use tonic::{transport::Server, Request, Response, Status, Streaming, Code};

use std::{fs, thread};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::FileExt;
use prost::Message;
use zip::write::FileOptions;
use iter_tools::Itertools;

use crate::cli_proto::modules_server::ModulesServer;
use crate::cli_proto::modules_server::Modules;
use crate::cli_proto::*;

#[derive(Parser)]
struct Cli {
    /// Host addr interface to listen to
    address: String,
    /// Port to listen to
    port: u16,
    modules_path: PathBuf,
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
        let mut success = true;
        let mut module_name = String::new();
        let rt_path = self.manager.lock().unwrap().watcher.dir.clone();
        if let Some(item) = streaming.message().await? {
            let full_path = rt_path.join(item.file_name.clone());
            let mut file = fs::File::create(full_path.clone())?;
            file.write_all(&item.runnable_bytes)?;
            println!("!! wrote {} bytes", item.runnable_bytes.len());
            while let Some(item) = streaming.message().await? {
                file.write_all(&item.runnable_bytes)?;
                println!("!! wrote {} bytes", item.runnable_bytes.len());
            }
            file.flush()?;

            let inputs: String = item.inputs;
            let outputs: String = item.outputs;
            let in_arr = inputs.split(",").join("', '");
            let out_arr = outputs.split(",").join("', '");
            let meta_contents = if in_arr.is_empty() {
                format!("{{ inputs: [], outputs: [] }}")
            } else {
                format!("{{ inputs: ['{}'], outputs: ['{}'] }}", in_arr, out_arr)
            };

            let file_name_part = PathBuf::from(item.file_name.clone());
            let local_name = file_name_part.file_stem().unwrap().to_str().unwrap().to_owned();
            let file_name = PathBuf::from(format!("{}.zip", local_name));
            let zip_path = rt_path.join(file_name);
            if let Ok(mut file) = fs::File::open(full_path.clone()) {
                if let Ok(zip_file) = fs::File::create(zip_path) {
                    let mut zip_writer = zip::ZipWriter::new(zip_file);
                    if let Ok(()) = zip_writer.start_file("meta.json", FileOptions::default()) {
                        zip_writer.write_all(&meta_contents.encode_to_vec()[..])?;
                    } else {
                        eprintln!("Cannot start file in zip: meta.json");
                    }
                    if let Ok(()) = zip_writer.start_file("runnable.wasm", FileOptions::default()) {
                        let mut buffer = vec![];
                        file.read_to_end(&mut buffer)?;
                        zip_writer.write_all(&buffer)?;
                    } else {
                        eprintln!("Cannot start file in zip: meta.json");
                    }
                    if let Err(err) = zip_writer.finish() {
                        eprintln!("Cannot finish writing zip bytes: {}", err);
                    }
                }
            } else {
                eprintln!("Cannot open WASM file to copy into zip file");
            }
            module_name = local_name;
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
            .or_else(|| Some(ModuleStatus::Undeployed))
            .unwrap();
        let reply = ModuleLoadReply {
            success: success && module_status.eq(&ModuleStatus::Deploy),
            error_message,
            time: 0,
        };
        Ok(Response::new(reply))
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
