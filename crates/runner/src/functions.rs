use crate::data::DataFrame;
use crate::runner::{CompilationUnit, Compiler, Executor};
use crate::watcher::DirectoryWatcher;

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::{Read, Seek};
use std::path::PathBuf;
use std::time::SystemTime;
use strum_macros::AsRefStr;
use thiserror::Error;
use zip::ZipArchive;

fn get_file_checksum(p: &PathBuf) -> Result<String, io::Error> {
    let mut file = fs::File::open(&p)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let new_checksum_arr = hasher.finalize();
    Ok(format!("{:x}", new_checksum_arr))
}

#[derive(Clone)]
pub struct Module {
    checksum: String,
    pub name: String,
    pub status: FunctionStatus,
    pub file_path: PathBuf,
    compilation: Option<CompilationUnit>,
}

pub struct ModuleHandle<'a> {
    pub name: String,
    compilation_unit: Option<CompilationUnit>,
    backreference: &'a FunctionManager,
}

impl<'a> ModuleHandle<'a> {
    pub fn run(&self, frame: &DataFrame) -> Result<DataFrame, String> {
        match self.backreference
            .executor
            .execute(&self.compilation_unit, frame) {
            Ok(dataframe) => {
                println!("Successfully executed fn {}", self.name);
                Ok(dataframe)
            },
            Err(err) => {
                eprintln!("Cannot execute fn named {} because {}", self.name, err);
                Err(format!("Cannot execute module fn named {}", self.name))
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum FunctionManagerError {
    #[error("Unavailable module {0:?}")]
    UnavailableModule(String),

    #[error("Error while compiling {0:?} because {1:?}")]
    CompilationError(String, String),
}

pub struct FunctionManager {
    pub watcher: DirectoryWatcher,
    module_map: HashMap<String, Module>,
    pub compiler: Compiler,
    pub executor: Executor,
}

impl FunctionManager {
    pub fn new(path: PathBuf) -> FunctionManager {
        let (compiler, executor) = crate::runner::new_pair();
        FunctionManager {
            watcher: DirectoryWatcher::new(path),
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
            .filter(|module| module.status.eq(&FunctionStatus::Deployed))
            .collect()
    }

    pub fn tick(&mut self) {
        let to_undeploy = self.deleted_functions();
        for item in to_undeploy {
            self.undeploy(&item).unwrap();
        }

        let dropped_files = self.watcher.run();
        for file_entry in dropped_files {
            let stem = file_entry.path.file_stem().unwrap();
            let module_name = stem.to_str().unwrap().to_owned();
            let next_status = FunctionStatus::from_string(&file_entry.next_status);

            if let Some(item) = self.module_map.get(&module_name.clone()) {
                println!("dropped file {}", file_entry.path.to_str().unwrap().to_string());
                match get_file_checksum(&file_entry.path) {
                    Ok(file_checksum) => {
                        if !file_checksum.eq(&item.checksum) {
                            println!("checksum {} differs from {}: going to reload fn", file_checksum, item.checksum.clone());
                            self.load(&module_name, &next_status, &file_checksum);
                        } else {
                            println!("same checksum for {} = {}: no reload", module_name, item.checksum.clone());
                        }
                    }
                    Err(error) => eprintln!(
                        "Cannot calculate checksum for module {} because {:?}",
                        module_name.clone(),
                        error
                    ),
                }
            } else {
                match get_file_checksum(&file_entry.path) {
                    Ok(file_checksum) => {
                        let item = Module {
                            checksum: file_checksum.clone(),
                            name: module_name.clone(),
                            status: FunctionStatus::Undeployed,
                            file_path: file_entry.path.clone(),
                            compilation: None,
                        };
                        self.module_map.insert(module_name.to_string(), item);
                        self.load(&module_name, &next_status, &file_checksum);
                    }
                    Err(error) => eprintln!(
                        "Cannot calculate checksum for module {} because {:?}",
                        module_name.clone(),
                        error
                    ),
                }
            }
        }
        self.watcher.remove_next_states();
    }

    fn deleted_functions(&self) -> Vec<String> {
        let mut to_undeploy = vec![];
        for (module_name, module) in self.module_map.iter() {
            if !module.file_path.exists() {
                to_undeploy.push(module_name.clone());
            }
        }
        to_undeploy
    }

    pub fn get_handle(&self, module_name: &String) -> Option<ModuleHandle> {
        if let Some(module) = self.module_map.get(module_name) {
            let module_status = module.status;
            if module_status.eq(&FunctionStatus::Deployed) || module_status.eq(&FunctionStatus::Deploy) {
                if let Some(cu) = module.compilation.clone() {
                    return Some(ModuleHandle {
                        name: module_name.clone(),
                        backreference: self,
                        compilation_unit: Some(cu),
                    })
                }
            }
        }
        None
    }

    fn get_fn_by_name(&mut self, name: &str) -> Option<Module> {
        return self.module_map.get(name).cloned();
    }

    pub fn load(&mut self, module_name: &String, new_status: &FunctionStatus, file_checksum: &String) {
        println!("Loading fn {} with status {}", module_name, new_status.as_string());
        let t_now = SystemTime::now();
        if let Some(module) = self.get_fn_by_name(&module_name.clone()) {
            match new_status {
                FunctionStatus::Deploy | FunctionStatus::Deployed => {
                    if let Err(err) = self.deploy(&*module.name, file_checksum.clone()) {
                        eprintln!("Cannot deploy fn {} because {}", module_name, err);
                    }
                },
                FunctionStatus::Undeploy | FunctionStatus::Undeployed => {
                    let module_name = module_name.clone();
                    let deploy_result = self.undeploy(&module_name);
                    if deploy_result.is_err() {
                        println!(
                            "Couldn't undeploy {} because: {}",
                            module_name,
                            match deploy_result.err().unwrap() {
                                FunctionManagerError::UnavailableModule(module_name) =>
                                    format!("The module {} is not available anymore", module_name),
                                _ => "Unknown error".to_string(),
                            }
                        );
                    } else {
                        println!("Correctly deployed {}", module_name);
                    }
                },
                FunctionStatus::Redeploy => {
                    self.undeploy(module_name).unwrap();
                    self.deploy(module_name, file_checksum.clone()).unwrap();
                },
            }
        } else {
            eprintln!("Cannot load a fn not existent in memory, check consistency");
            return;
        }
        println!(
            "Modified module {} in {}ms",
            module_name.clone(),
            t_now.elapsed().unwrap().as_millis()
        );
    }

    fn deploy(&mut self, module_name: &str, checksum: String) -> Result<(), FunctionManagerError> {
        let module_map = self.running_modules_map();
        if let Some(module) = module_map.get(&module_name.to_owned()) {
            if let Ok(mut file) = fs::File::open(module.file_path.clone()) {
                let compilation_unit_result = self.compiler.compile(&mut file);
                if compilation_unit_result.is_err() {
                    return Err(FunctionManagerError::CompilationError(
                        module_name.to_owned(),
                        format!(
                            "couldn't JIT compile WASM: {:?}",
                            compilation_unit_result.err().unwrap()
                        ),
                    ));
                }
                let compilation = Some(compilation_unit_result.unwrap());
                self.module_map.insert(module_name.to_owned(), Module { status: FunctionStatus::Deployed, compilation, checksum, ..module.clone() });
                Ok(())
            } else {
                Err(FunctionManagerError::UnavailableModule(
                    module_name.to_owned(),
                ))
            }
        } else {
            Err(FunctionManagerError::UnavailableModule(module_name.to_owned()))
        }
    }

    fn undeploy(&mut self, module_name: &String) -> Result<FunctionStatus, FunctionManagerError> {
        if let Some(module) = self.module_map.get(&module_name.clone()) {
            let module_path = module.file_path.clone();
            if !module_path.exists() {
                let _ = fs::remove_file(module_path);
                self.module_map.remove(module_name).unwrap();
            }
            Ok(FunctionStatus::Undeployed)
        } else {
            Err(FunctionManagerError::UnavailableModule(module_name.clone()))
        }
    }
}

fn open_zip(path: PathBuf) -> Result<ZipArchive<impl Read + Seek>, String> {
    if let Ok(file) = fs::File::open(path) {
        if let Ok(archive) = ZipArchive::new(file) {
            Ok(archive)
        } else {
            Err("Cannot open zip file from reader".to_string())
        }
    } else {
        Err("Cannot open zip file".to_string())
    }
}

#[derive(AsRefStr, PartialEq, Clone, Copy)]
pub enum FunctionStatus {
    Deploy,
    Deployed,
    Redeploy,
    Undeploy,
    Undeployed,
}

impl FunctionStatus {
    pub fn from_string(str: &str) -> FunctionStatus {
        match str {
            "deploy" => FunctionStatus::Deploy,
            "undeploy" => FunctionStatus::Undeploy,
            "running" => FunctionStatus::Deployed,
            "undeployed" => FunctionStatus::Undeployed,
            "redeploy" => FunctionStatus::Redeploy,
            _ => FunctionStatus::Undeployed
        }
    }

    pub fn as_string(&self) -> String {
        let str = match self {
            FunctionStatus::Deploy => "deploy",
            FunctionStatus::Deployed => "deployed",
            FunctionStatus::Undeploy => "undeploy",
            FunctionStatus::Undeployed => "undeployed",
            FunctionStatus::Redeploy => "redeploy"
        };
        String::from(str)
    }
}
