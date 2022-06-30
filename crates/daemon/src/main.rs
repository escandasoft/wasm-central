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
use std::{fs, thread};
use zip::write::FileOptions;

use crate::fn_proto::functions_server::Functions;
use crate::fn_proto::functions_server::FunctionsServer;
use crate::fn_proto::*;

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

pub struct MyFunctions {
    manager: Arc<Mutex<FunctionManager>>,
}

impl MyFunctions {
    pub fn new(manager: Arc<Mutex<FunctionManager>>) -> MyFunctions {
        MyFunctions { manager }
    }
}

#[tonic::async_trait]
impl Functions for MyFunctions {
    async fn list(
        &self,
        request: Request<FunctionListRequest>,
    ) -> Result<Response<FunctionListReply>, Status> {
        let items = self
            .manager
            .lock()
            .unwrap()
            .running_modules()
            .iter()
            .map(|loaded_module| {
                let module_status = loaded_module.status;
                FunctionListReplyItem {
                    name: String::from(&loaded_module.name),
                    status: module_status.as_string(),
                    successes: 0,
                    failures: 0,
                    total_messages: 0,
                    fail_rate_per_minute: 0.0,
                }
            })
            .collect::<Vec<FunctionListReplyItem>>();
        Ok(Response::new(FunctionListReply {
            items: items.clone(),
            item_no: items.len() as i32,
        }))
    }

    async fn load(
        &self,
        request: Request<Streaming<FunctionLoadPartRequest>>,
    ) -> Result<Response<FunctionLoadReply>, Status> {
        let mut streaming = request.into_inner();
        let success = true;
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
            let local_name = file_name_part
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
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
            .or_else(|| Some(FunctionStatus::Undeployed))
            .unwrap();
        let reply = FunctionLoadReply {
            success: success && module_status.eq(&FunctionStatus::Deploy),
            error_message,
            time: 0,
        };
        Ok(Response::new(reply))
    }

    async fn unload(
        &self,
        request: Request<FunctionUnloadRequest>,
    ) -> Result<Response<FunctionUnloadReply>, Status> {
        return Ok(Response::new(FunctionUnloadReply {
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

    let modules_server = FunctionsServer::new(MyFunctions::new(mgr.clone()));
    let bootstrap_future = Server::builder().add_service(modules_server).serve(faddr);
    println!("Server ready at {}", blue.apply_to(faddr));

    let mgr = Arc::clone(&mgr);
    thread::spawn(move || loop {
        mgr.lock().unwrap().tick();
        thread::sleep(Duration::from_millis(MODULE_MANAGER_LOOP_WAIT));
    });
    bootstrap_future.await?;
    return Ok(());
}
