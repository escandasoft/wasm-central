use std::sync::atomic::{AtomicBool, Ordering};
use std::vec::Vec;

use tonic::{transport::Server, Request, Response, Status, Streaming};
use clap::Parser;
use console::Style;

use crate::datatx_proto::modules_server::{Modules, ModulesServer};
use crate::datatx_proto::*;

use wasm_central::watcher::Watcher;

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

pub struct ModuleManager {
    path: std::path::PathBuf,
    watcher_thread: Option<std::thread::JoinHandle<()>>,
    watcher: Watcher,
    is_running: AtomicBool,
}

impl ModuleManager {
    pub fn on_deployable_item(&self, p: &std::path::Path, next_status: &String) -> () {
        
    }

    pub fn start(&self) -> () {
        self.watcher_thread = Some(std::thread::spawn(move || {
            if self.is_running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).unwrap() {
                while self.is_running.fetch_and(true, Ordering::SeqCst) {
                    let callback = |p: &std::path::Path, next_status: &String| { self.on_deployable_item(p, next_status) };
                    self.watcher.run(callback);
                    std::thread::yield_now();
                    std::thread::sleep_ms(1000);
                }
            }
        }));
    }

    pub fn stop(&self) {
        if self.is_running.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst).expect("cannot use stop flag") {
            self.watcher_thread.unwrap().join();
        }
    }
}

#[derive(Debug)]
pub struct LoadedModule {
    name: String,
    path: String,
    status: String,
    processes: Vec<i32>
}

#[derive(Debug)]
pub struct MyModules {
    hot_folder_path: std::path::PathBuf,
    mutex: std::sync::Mutex<u8>,
    loaded_modules: Vec<LoadedModule>,
    is_running: AtomicBool
}

#[tonic::async_trait]
impl Modules for MyModules {
    async fn list(&self, request: Request<Empty>) -> Result<Response<ModuleListReply>, Status> {
        match self.mutex.lock() {
            Ok(lock) => {
                let items = self.loaded_modules.iter().map(|loaded_module| {
                    return ModuleListReplyItem {
                        name: String::from(&loaded_module.name),
                        status: loaded_module.status.clone(),
                        successes: 0,
                        failures: 0,
                        total_messages: 0,
                        fail_rate_per_minute: 0.0
                    }
                }).collect::<Vec<ModuleListReplyItem>>();
                return Ok(Response::new(ModuleListReply {
                    items: items,
                    item_no: self.loaded_modules.len() as i32,
                }));
            },
            _ => { return Result::Err(Status::new(tonic::Code::from(500), "cannot acquire lock")); }
        }
    }

    async fn load(&self, request: Request<Streaming<ModuleLoadPartRequest>>) -> Result<Response<ModuleLoadReply>, Status> {
        return Ok(Response::new(ModuleLoadReply {
            success: false,
            error_message: None,
            time: 0
        }));
    }

    async fn replace(&self, request: Request<ModuleReplaceRequest>) -> Result<Response<ModuleReplaceReply>, Status> {
        return Ok(Response::new(ModuleReplaceReply {
            success: false,
            error_message: None,
            time: 0
        }))
    }

    async fn unload(&self, request: Request<ModuleUnloadRequest>) -> Result<Response<ModuleUnloadReply>, Status> {
        return Ok(Response::new(ModuleUnloadReply {
            success: false,
            error_message: None,
            unloaded_module_name: String::from("proc"),
            time: 0
        }))
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

    let watcher = Watcher::new(&path);
    let service = MyModules {
        hot_folder_path: path.clone(),
        is_running: AtomicBool::new(false),
        loaded_modules: vec!(),
        mutex: std::sync::Mutex::new(0)
    };

    let bootstrap_future = Server::builder()
        .add_service(ModulesServer::new(service))
        .serve(faddr);
    println!("Server ready at {}", blue.apply_to(faddr));
    bootstrap_future.await?;
    return Ok(());
}
