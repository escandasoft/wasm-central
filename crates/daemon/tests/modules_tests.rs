use wasm_central_runner::modules::ModuleManager;

use std::fs;
use std::path::PathBuf;

#[test]
fn test_file_handling() {
    let path = PathBuf::from("./");

    let full_path = path.join("./module.zip");
    let rt_path = path.join("target/runtime/");

    fs::remove_dir_all(rt_path.clone())?;
    fs::create_dir(rt_path.clone())?;

    let mut module_manager = ModuleManager::new(rt_path.clone());

    module_manager.tick();
    assert_eq!(0, module_manager.running_modules().len());

    let module_path = rt_path.join("./module.zip");
    fs::copy(full_path.clone(), module_path.clone())
        .expect("Cannot copy module.zip into ./target/runtime/");

    println!("Trying to load module through manager");
    module_manager.tick();
    assert_eq!(1, module_manager.running_modules().len());

    fs::remove_file(module_path.clone()).expect("Cannot remove module file");
    module_manager.tick();

    let module_name = "module".to_string();
    let module_handle = module_manager.get_handle(&module_name);
    assert_eq!(false, module_handle.is_some());
    assert_eq!(0, module_manager.running_modules().len());

    fs::copy(full_path.clone(), module_path.clone())
        .expect("Cannot copy module.zip into ./target/runtime/");
    module_manager.tick();
}
