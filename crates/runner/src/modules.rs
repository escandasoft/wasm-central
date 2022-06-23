use crate::data::DataFrame;
use crate::runner::{CompilationUnit, Compiler, Executor};
use crate::watcher::DirectoryWatcher;

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek};
use std::path::PathBuf;
use std::time::SystemTime;
use strum_macros::AsRefStr;
use thiserror::Error;
use zip::ZipArchive;

fn get_file_checksum(p: &PathBuf) -> String {
    let mut file = fs::File::open(&p).expect("Cannot open file to calculate checksum");
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).expect("Cannot copy contents into Digest for checksum");
    let new_checksum_arr = hasher.finalize();
    format!("{:x}", new_checksum_arr)
}

#[derive(Clone)]
pub struct Module {
    checksum: String,
    pub name: String,
    pub status: ModuleStatus,
    pub file_path: PathBuf,
    compilation: Option<CompilationUnit>,
}

pub struct ModuleHandle<'a> {
    pub name: String,
    compilation_unit: CompilationUnit,
    backreference: &'a ModuleManager,
}

impl<'a> ModuleHandle<'a> {
    pub fn run(&self, frame: &DataFrame) -> Result<DataFrame, String> {
        self.backreference
            .executor
            .execute(&self.compilation_unit, frame)
    }
}

#[derive(Error, Debug)]
pub enum ModuleManagerError {
    #[error("Unavailable module {0:?}")]
    UnavailableModule(String),

    #[error("Error while compiling {0:?} because {1:?}")]
    CompilationError(String, String),
}

pub struct ModuleManager {
    watcher: DirectoryWatcher,
    module_map: HashMap<String, Module>,
    pub compiler: Compiler,
    pub executor: Executor,
}

impl ModuleManager {
    pub fn new(path: PathBuf) -> ModuleManager {
        let (compiler, executor) = crate::runner::new_pair();
        ModuleManager {
            watcher: DirectoryWatcher::new(path.clone()),
            module_map: HashMap::new(),
            compiler,
            executor,
        }
    }

    pub fn running_modules_map(&self) -> HashMap<String, Module> {
        self.module_map.clone()
    }

    pub fn running_modules(&self) -> Vec<&Module> {
        self.module_map
            .iter()
            .map(|(_name, module)| module)
            .filter(|module| module.status.eq(&ModuleStatus::Deployed))
            .collect()
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
                    self.load(&module_name, &next_status);
                }
            } else {
                let item = Module {
                    checksum: get_file_checksum(&file_entry.path),
                    name: module_name.clone(),
                    status: ModuleStatus::Undeployed,
                    file_path: file_entry.path.clone(),
                    compilation: None,
                };
                self.module_map.insert(module_name.to_string(), item);
                self.load(&module_name, &next_status);
            }
        }
        let to_undeploy = self.get_to_undeploy();
        for item in to_undeploy {
            self.undeploy(&item).unwrap();
        }
    }

    fn get_to_undeploy(&self) -> Vec<String> {
        let mut to_undeploy = vec![];
        for (module_name, module) in self.module_map.iter() {
            if !module.file_path.exists() {
                to_undeploy.push(module_name.clone());
            }
        }
        to_undeploy
    }

    pub fn get_handle(&self, module_name: &String) -> Option<ModuleHandle> {
        let module_opt = self.module_map.get(module_name);
        if module_opt.is_none() {
            None
        } else {
            let module_status = module_opt.unwrap().status;
            println!(
                "!! found module?: {}, status: {}",
                module_opt.is_some(),
                module_status.as_ref()
            );
            if module_opt.is_some() && module_status.eq(&ModuleStatus::Deployed) {
                let module = module_opt.unwrap();
                let cu = module.compilation.as_ref().unwrap();
                Some(ModuleHandle {
                    name: module_name.clone(),
                    backreference: self,
                    compilation_unit: cu.clone(),
                })
            } else {
                None
            }
        }
    }

    pub fn load(&mut self, module_name: &String, new_status: &ModuleStatus) {
        println!("Starting to load module {}", module_name.clone());
        let t_now = SystemTime::now();
        let module_opt = self.module_map.get(&module_name.clone());
        if module_opt.is_none() {
            return;
        }

        let module = module_opt.unwrap();
        match module.status {
            ModuleStatus::Deploy => {
                if new_status.eq(&ModuleStatus::Undeploy)
                    || new_status.eq(&ModuleStatus::Undeployed)
                {
                    self.undeploy(module_name).unwrap();
                }
            }
            ModuleStatus::Deployed => {}
            ModuleStatus::Undeploy => {}
            ModuleStatus::Undeployed => {
                let module_name = module_name.clone();
                let deploy_result = self.deploy(&module_name);
                if deploy_result.is_err() {
                    println!(
                        "Couldn't deploy {} because: {}",
                        module_name,
                        match deploy_result.err().unwrap() {
                            ModuleManagerError::UnavailableModule(module_name) => String::from("The module is not available anymore"),
                            ModuleManagerError::CompilationError(module_name, err_msg) => err_msg
                        }
                    );
                } else {
                    println!("Correctly deployed {}", module_name);
                }
            }
        }
        println!(
            "Modified module {} in {}ms",
            module_name.clone(),
            t_now.elapsed().unwrap().as_millis()
        );
    }

    fn deploy(&mut self, module_name: &String) -> Result<ModuleStatus, ModuleManagerError> {
        let module_map = self.running_modules_map();
        let mod_opt = module_map.get(&module_name.clone());
        if mod_opt.is_none() {
            return Err(ModuleManagerError::UnavailableModule(module_name.clone()));
        }
        let module = mod_opt.unwrap();

        let meta_zip_result = open_zip(module.file_path.clone());
        let runnable_zip_result = open_zip(module.file_path.clone());

        if meta_zip_result.is_err() || runnable_zip_result.is_err() {
            return Err(ModuleManagerError::CompilationError(
                module_name.clone(),
                "cannot open zip archive".to_string(),
            ));
        }

        let mut meta_archive = meta_zip_result.unwrap();
        let meta_file_opt = meta_archive.by_name("meta.json");
        if meta_file_opt.is_err() {
            return Err(ModuleManagerError::CompilationError(
                module_name.clone(),
                "cannot find meta.json file in zip archive".to_string(),
            ));
        }

        let mut runnable_archive = runnable_zip_result.unwrap();
        let runnable_file_opt = runnable_archive.by_name("runnable.wasm");
        if runnable_file_opt.is_err() {
            return Err(ModuleManagerError::CompilationError(
                module_name.clone(),
                "cannot find runnable.wasm file in zip archive".to_string(),
            ));
        }

        let _meta_file = meta_file_opt.unwrap();
        let mut runnable_file = runnable_file_opt.unwrap();

        let compilation_unit_result = self.compiler.compile(&mut runnable_file);
        if compilation_unit_result.is_err() {
            return Err(ModuleManagerError::CompilationError(
                module_name.clone(),
                format!(
                    "couldn't JIT compile WASM: {:?}",
                    compilation_unit_result.err().unwrap()
                ),
            ));
        }
        self.change_status(&module_name.clone(), &module, ModuleStatus::Deploy);
        Ok(ModuleStatus::Deployed)
    }

    fn undeploy(&mut self, module_name: &String) -> Result<ModuleStatus, ModuleManagerError> {
        let mod_opt = self.module_map.get(&module_name.clone());
        if mod_opt.is_none() {
            Err(ModuleManagerError::UnavailableModule(module_name.clone()))
        } else {
            let module = mod_opt.unwrap();
            let module_path = module.file_path.clone();
            if !module_path.exists() {
                let _ = fs::remove_file(module_path);
                self.module_map.remove(module_name).unwrap();
            }
            Ok(ModuleStatus::Undeployed)
        }
    }

    fn change_status(&mut self, module_name: &String, module: &Module, status: ModuleStatus) {
        let module_replacement = Module {
            status: status,
            ..module.clone()
        };
        self.module_map
            .insert(module_name.clone(), module_replacement)
            .unwrap();
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

#[derive(AsRefStr, PartialEq, Clone, Copy)]
pub enum ModuleStatus {
    Deploy,
    Deployed,
    Undeploy,
    Undeployed,
}

impl ModuleStatus {
    fn from_string(str: &String) -> ModuleStatus {
        match str.as_str() {
            "deploy" => ModuleStatus::Deploy,
            "undeploy" => ModuleStatus::Undeploy,
            "running" => ModuleStatus::Deployed,
            "undeployed" => ModuleStatus::Undeployed,
            _ => ModuleStatus::Undeployed,
        }
    }
}
