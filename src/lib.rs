pub mod watcher {

use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ChangedEntryStatus {
    DEPLOY, UNDEPLOY, RUNNING, UNDEPLOYED
}

#[derive(Debug)]
pub struct ChangedEntry {
    path: PathBuf,
    status: ChangedEntryStatus
}

#[derive(Debug)]
pub struct WatcherEntry {
    pub path: PathBuf,
    pub next_status: String
}

#[derive(Debug)]
pub struct DirectoryWatcher {
    pub dir: PathBuf
}

impl DirectoryWatcher {
    pub fn new(p: PathBuf) -> Self {
        Self { dir: p }
    }

    pub fn run(&self) -> Vec<WatcherEntry> {
        let dir = std::fs::read_dir(&self.dir).unwrap();
        let mut dropped_files = vec!();
        for result in dir {
            let file = result.expect("result needed");
            println!("!! found file at {}", String::from(file.file_name().to_str().unwrap()));
            let path = file.path();
            let ext = path.extension();
            if ext.is_some() && ext.unwrap().eq("zip") {
                let name = path.file_stem();
                let mut status_str = "deploy";
                for alternate_status in ["deploy", "undeploy", "running", "undeployed"] {
                    let part_path = format!("{:?}.{}", name.expect("name"), &alternate_status);
                    let rel_path = path.parent().unwrap().join(part_path);
                    if Path::new(&rel_path).exists() {
                        status_str = alternate_status;
                    }
                }
                let p = Path::new(&path);
                let pbuf = p.to_path_buf();
                let next_status = String::from(status_str);
                println!("!! added dropped file to {} with {} status", pbuf.display(), next_status);
                dropped_files.push(WatcherEntry { path: pbuf, next_status: next_status});
            }
        }
        return dropped_files;
    }
 }
 
}

pub mod data {

pub struct DataFrame {
}

}

pub mod runner {

use std::io::Read;
    use std::ops::Deref;
    use std::sync::Arc;
    use fork::Fork;
    use wasmer::{Store, Module, Instance, Value, imports, Universal, Cranelift, UniversalEngine, Engine};
    use crate::data::DataFrame;

    #[derive(Clone)]
pub struct CompilationUnit {
    module: Module
}

pub struct Compiler {
    pub engine: Arc<UniversalEngine>,
    store: Store,
}

impl Compiler {
    pub fn new() -> Compiler {
        let comp_config = Cranelift::default();
        let engine = Arc::new(Universal::new(comp_config)
            .engine());
        let store = Store::new(engine.deref());
        Compiler {
            engine,
            store
        }
    }

    pub fn compile(&self, reader: &mut impl Read) -> Result<CompilationUnit, String> {
        let mut buff = vec![];
        reader.read_to_end(&mut buff).expect("Cannot use reader during compilation");
        let module_result = Module::new(&self.store, buff);
        Ok(CompilationUnit { module: module_result.unwrap() })
    }
}

pub struct Executor {
    pub engine: Arc<UniversalEngine>
}

impl Executor {
    pub fn new(&self, engine: Arc<UniversalEngine>) -> Executor {
        Executor {
            engine
        }
    }

    pub fn execute(&self, compilation_unit: CompilationUnit, frame: &DataFrame) -> Result<DataFrame, String> {
        match fork::fork() {
            Ok(Fork::Child) => {
                let imports = imports! {};
                let instance_result = Instance::new(&compilation_unit.module, &imports);
                if instance_result.is_ok() {
                    let instance = instance_result.unwrap();
                    let function_result = instance.exports.get_function("main");
                    function_result.unwrap().call(&[]);
                }
            },
            Ok(Fork::Parent(child)) => {

            },
            Err(_) => {
                println!()
            }
        }
        Ok(DataFrame {})
    }
}

}

pub mod modules {

use std::io::{Seek, Read};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;
use sha2::{Sha256, Digest};
use sha2::digest::generic_array::{GenericArray, ArrayLength};
use std::fs;
use std::sync::Mutex;
    use std::time::SystemTime;
    use zip::read::ZipFile;
use zip::ZipArchive;
use crate::modules::ModuleStatus::{DEPLOY, DEPLOYED, UNDEPLOY, UNDEPLOYED};
use crate::runner::CompilationUnit;
use strum_macros::AsRefStr;

    fn get_file_checksum(p: &PathBuf) -> String {
    let mut file = fs::File::open(&p)
        .expect("Cannot open file to calculate checksum");
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)
        .expect("Cannot copy contents into Digest for checksum");
    let new_checksum_arr = hasher.finalize();
    format!("{:x}", new_checksum_arr)
}

pub struct ModuleHandle {
    name: String
}

pub struct ModuleManager {
    path: PathBuf,
    watcher: crate::watcher::DirectoryWatcher,
    module_map: HashMap<String, DeployedItem>,
    pub compiler: crate::runner::Compiler,
}

impl ModuleManager {
    fn new(path: PathBuf) -> ModuleManager {
        ModuleManager {
            path: path.clone(),
            watcher: crate::watcher::DirectoryWatcher::new(path.clone()),
            module_map: HashMap::new(),
            compiler: crate::runner::Compiler::new(),
        }
    }

    pub fn tick(&mut self) {
        let dropped_files = self.watcher.run();
        for file_entry in dropped_files {
            let stem = file_entry.path.file_stem().unwrap();
            let module_name = String::from(stem.to_str().unwrap());
            let item_opt = self.module_map.get(&module_name.clone());
            let next_status = ModuleStatus::from_string(&file_entry.next_status);
            
            if item_opt.is_some() {
                let item = item_opt.unwrap();
                let new_checksum = get_file_checksum(&file_entry.path);
                if !new_checksum.eq(&item.checksum) {
                    self.load(&module_name, &file_entry.path, &next_status);
                }
            } else {
                let item = DeployedItem {
                    checksum: get_file_checksum(&file_entry.path),
                    name: module_name.clone(),
                    status: ModuleStatus::UNDEPLOYED,
                    file_path: file_entry.path.clone(),
                    compilation: None
                };
                self.module_map.insert(module_name.to_string(), item);
            }
            println!("Starting to load module {}", module_name.clone());
            let t_now = SystemTime::now();
            self.load(&module_name, &file_entry.path, &next_status);
            println!("Loaded module {} in {}ms", module_name.clone(), t_now.elapsed().unwrap().as_millis());
        }
    }

    pub fn get_handle(&self, module_name: &String) -> Option<ModuleHandle> {
        let module_opt = self.module_map.get(module_name);
        if module_opt.is_none() {
            None
        } else {
            let module_status = module_opt.unwrap().status;
            println!("!! found module?: {}, status: {}", module_opt.is_some(), module_status.as_ref());
            if module_opt.is_some() && module_status.eq(&DEPLOYED) {
                Some(ModuleHandle {
                    name: module_name.clone()
                })
            } else {
                None
            }
        }
    }

    pub fn load(&mut self, module_name: &String, path: &PathBuf, new_status: &ModuleStatus) {
        let module_opt = self.module_map.get(&module_name.clone());
        if module_opt.is_none() {
            return;
        }

        let module = module_opt.unwrap();
        match module.status {
            UNDEPLOYED => {
                let module_name = module_name.clone();
                let deploy_result = self.deploy(&module_name);
                if deploy_result.is_err() {
                    println!("Couldn't deploy {} because: {}", module_name, deploy_result.err().unwrap());
                } else {
                    println!("Correctly deployed {}", module_name);
                }
            },
            UNDEPLOY => {
            },
            DEPLOYED => {
                if new_status.eq(&UNDEPLOY) || new_status.eq(&UNDEPLOYED) {
                    self.undeploy(module_name);
                }
            },
            UNDEPLOYED => {
                if new_status.eq(&DEPLOY) || new_status.eq(&DEPLOYED) {

                }
            }
            _ => {}
        }
    }

    fn deploy(&mut self, module_name: &String) -> Result<bool, String> {
        let mod_opt = self.module_map.get(module_name).cloned();
        if mod_opt.is_none() {
            return Err(format!("Cannot find module by name {} during deploy", module_name));
        }
        let mut module = mod_opt.unwrap();

        let meta_zip_result = open_zip(module.file_path.clone());
        let runnable_zip_result = open_zip(module.file_path.clone());

        if meta_zip_result.is_err() || runnable_zip_result.is_err() {
            return Err("Cannot open zip archive".to_string());
        }

        let mut meta_archive = meta_zip_result.unwrap();
        let meta_file_opt = meta_archive.by_name("meta.json");
        if meta_file_opt.is_err() {
            return Err("Cannot find meta.json file in zip archive".to_string());
        }

        let mut runnable_archive = runnable_zip_result.unwrap();
        let runnable_file_opt = runnable_archive.by_name("runnable.wasm");
        if runnable_file_opt.is_err() {
            return Err("Cannot find runnable.wasm file in zip archive".to_string());
        }

        let meta_file = meta_file_opt.unwrap();
        let mut runnable_file = runnable_file_opt.unwrap();

        let compilation_unit_result = self.compiler.compile(&mut runnable_file);
        if compilation_unit_result.is_err() {
            return Err(format!("Cannot compile WASM: {}", compilation_unit_result.err().unwrap()));
        }
        self.module_map.insert(module_name.clone(), DeployedItem {
            checksum: module.checksum.clone(),
            name: module_name.clone(),
            status: DEPLOYED,
            file_path: module.file_path.clone(),
            compilation: Some(compilation_unit_result.unwrap()),
        }).unwrap();

        Ok(true)
    }

    fn undeploy(&mut self, module_name: &String) -> Result<ModuleStatus, String> {
        let mod_opt = self.module_map.get(module_name);
        if mod_opt.is_none() {
            return Err(format!("Cannot find module by name {} during undeploy", module_name));
        }
        let module = mod_opt.unwrap();
        fs::remove_file(module.file_path.clone()).unwrap();
        self.module_map.remove(module_name).unwrap();
        Ok(UNDEPLOYED)
    }
}

fn open_zip(path: PathBuf) -> Result<ZipArchive<impl Read + Seek>, String> {
    let file = fs::File::open(path);
    if file.is_err() {
        return Err("Cannot open zip file".to_string());
    }

    let archive = ZipArchive::new(file.unwrap());
    if archive.is_err() {
        return Err("Cannot open zip file from reader".to_string());
    }

    Ok(archive.unwrap())
}

#[derive(AsRefStr,PartialEq,Clone,Copy)]
pub enum ModuleStatus {
    DEPLOY, DEPLOYED, UNDEPLOY, UNDEPLOYED
}

impl ModuleStatus {
    fn from_string(str: &String) -> ModuleStatus {
        match str.as_str() {
            "deploy" => DEPLOY,
            "undeploy" => UNDEPLOY,
            "running" => DEPLOYED,
            "undeployed" => UNDEPLOYED,
            _ => UNDEPLOYED
        }
    }
}

#[derive(Clone)]
struct DeployedItem {
    checksum: String,
    name: String,
    status: ModuleStatus,
    file_path: PathBuf,
    compilation: Option<CompilationUnit>
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Write;
    use std::ops::Deref;
    use crate::runner::Executor;

    #[test]
    fn check_file_handling() {
        let path = PathBuf::from("./");

        let full_path = path.join("module.zip");
        let rt_path = path.join("target/runtime/");

        fs::remove_dir_all(rt_path.clone()).unwrap();
        fs::create_dir(rt_path.clone()).unwrap();

        let mut module_manager = ModuleManager::new(rt_path.clone());
        
        module_manager.tick();
        assert_eq!(0, module_manager.module_map.len());

        fs::copy(full_path, rt_path.join("module.zip"))
            .expect("Cannot copy module.zip into target/modules/");

        module_manager.tick();
        assert_eq!(1, module_manager.module_map.len());

        let module_name = "module".to_string();
        let module_handle = module_manager.get_handle(&module_name);
        assert_eq!(true, module_handle.is_some());
        Executor {
            engine: module_manager.compiler.engine
        };
    }
}

}