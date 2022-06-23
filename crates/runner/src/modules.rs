use crate::watcher::DirectoryWatcher;
use crate::runner::{Compiler, CompilationUnit};
use sha2::digest::generic_array::{ArrayLength, GenericArray};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::time::SystemTime;
use strum_macros::AsRefStr;
use zip::read::ZipFile;
use zip::ZipArchive;

fn get_file_checksum(p: &PathBuf) -> String {
    let mut file = fs::File::open(&p).expect("Cannot open file to calculate checksum");
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)
        .expect("Cannot copy contents into Digest for checksum");
    let new_checksum_arr = hasher.finalize();
    format!("{:x}", new_checksum_arr)
}

#[derive(Clone)]
pub struct LoadedModule {
    checksum: String,
    pub name: String,
    pub status: ModuleStatus,
    pub file_path: PathBuf,
    compilation: Option<CompilationUnit>,
}

pub struct ModuleHandle<'a> {
    pub name: String,
    backreference: &'a ModuleManager,
}

pub struct ModuleManager {
    path: PathBuf,
    watcher: DirectoryWatcher,
    module_map: HashMap<String, LoadedModule>,
    pub compiler: Compiler,
}

impl ModuleManager {
    pub fn new(path: PathBuf) -> ModuleManager {
        ModuleManager {
            path: path.clone(),
            watcher: DirectoryWatcher::new(path.clone()),
            module_map: HashMap::new(),
            compiler: Compiler::new(),
        }
    }

    pub fn running_modules(&self) -> Vec<&LoadedModule> {
        self.module_map
            .iter()
            .map(|(name, module)| module)
            .filter(|module| module.status.eq(&ModuleStatus::deployed))
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
                let item = LoadedModule {
                    checksum: get_file_checksum(&file_entry.path),
                    name: module_name.clone(),
                    status: ModuleStatus::undeployed,
                    file_path: file_entry.path.clone(),
                    compilation: None,
                };
                self.module_map.insert(module_name.to_string(), item);
                self.load(&module_name, &next_status);
            }
        }
        let mut to_undeploy = self.get_to_undeploy();
        for item in to_undeploy {
            self.undeploy(&item);
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
            if module_opt.is_some() && module_status.eq(&ModuleStatus::deployed) {
                Some(ModuleHandle {
                    name: module_name.clone(),
                    backreference: self
                })
            } else {
                None
            }
        }
    }

    pub fn load(&mut self, module_name: &String, new_status: &ModuleStatus) {
        println!("Starting to load module {}", module_name.clone());
        let t_now = SystemTime::now();
        let mut module_opt = self.module_map.get(&module_name.clone());
        if module_opt.is_none() {
            return;
        }

        let mut module = module_opt.unwrap();
        match module.status {
            UNDEPLOYED => {
                let module_name = module_name.clone();
                let deploy_result = self.deploy(&module_name);
                if deploy_result.is_err() {
                    println!(
                        "Couldn't deploy {} because: {}",
                        module_name,
                        deploy_result.err().unwrap()
                    );
                } else {
                    println!("Correctly deployed {}", module_name);
                }
            }
            UNDEPLOY => {}
            DEPLOYED => {
                if new_status.eq(&ModuleStatus::undeploy) || new_status.eq(&ModuleStatus::undeployed) {
                    self.undeploy(module_name);
                }
            }
            UNDEPLOYED => if new_status.eq(&ModuleStatus::deploy) || new_status.eq(&ModuleStatus::deployed) {},
            _ => {}
        }
        println!(
            "Modified module {} in {}ms",
            module_name.clone(),
            t_now.elapsed().unwrap().as_millis()
        );
    }

    fn deploy(&mut self, module_name: &String) -> Result<ModuleStatus, String> {
        let mod_opt = self.module_map.get(module_name).cloned();
        if mod_opt.is_none() {
            return Err(format!(
                "Cannot find module by name {} during deploy",
                module_name
            ));
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
            return Err(format!(
                "Cannot compile WASM: {}",
                compilation_unit_result.err().unwrap()
            ));
        }
        self.module_map
            .insert(
                module_name.clone(),
                LoadedModule {
                    status: ModuleStatus::deployed,
                    ..module.clone()
                },
            )
            .unwrap();

        Ok(ModuleStatus::deployed)
    }

    fn undeploy(&mut self, module_name: &String) -> Result<ModuleStatus, String> {
        let mod_opt = self.module_map.get(module_name);
        if mod_opt.is_none() {
            return Err(format!(
                "Cannot find module by name {} during undeploy",
                module_name
            ));
        }
        let module = mod_opt.unwrap();
        let module_path = module.file_path.clone();
        self.module_map
            .insert(
                module_name.clone(),
                LoadedModule {
                    status: ModuleStatus::undeployed,
                    ..module.clone()
                },
            )
            .unwrap();
        if !module_path.exists() {
            fs::remove_file(module_path.clone());
            self.module_map.remove(module_name).unwrap();
        }
        Ok(ModuleStatus::undeployed)
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
    deploy,
    deployed,
    undeploy,
    undeployed,
}

impl ModuleStatus {
    fn from_string(str: &String) -> ModuleStatus {
        match str.as_str() {
            "deploy" => ModuleStatus::deploy,
            "undeploy" => ModuleStatus::undeploy,
            "running" => ModuleStatus::deployed,
            "undeployed" => ModuleStatus::undeployed,
            _ => ModuleStatus::undeployed,
        }
    }
}
