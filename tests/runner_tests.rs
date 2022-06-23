#[path = "../src/modules.rs"]
mod modules;
#[path = "../src/runner.rs"]
mod runner;

use modules::ModuleManager;
use runner::{Compiler, Executor};

use std::fs;
use std::io::Write;
use std::ops::Deref;
use std::path::PathBuf;

#[test]
fn test_executor_basics() {
    let path = PathBuf::from("./");

    let full_path = path.join("module.zip");
    let rt_path = path.join("target/runtime/");

    fs::remove_dir_all(rt_path.clone()).unwrap();
    fs::create_dir(rt_path.clone()).unwrap();

    let mut module_manager = ModuleManager::new(rt_path.clone());

    let module_path = rt_path.join("module.zip");
    fs::copy(full_path.clone(), module_path.clone())
        .expect("Cannot copy module.zip into ./target/runtime/");

    module_manager.tick();
    assert_eq!(1, module_manager.running_modules().len());

    let module_name = "module".to_string();
    let module_handle = module_manager.get_handle(&module_name);
    assert_eq!(true, module_handle.is_some());
    assert_eq!(1, module_manager.running_modules().len());
}