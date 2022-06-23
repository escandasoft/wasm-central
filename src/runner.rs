#[path = "./data.rs"]
mod data;

use data::DataFrame;
use fork::Fork;
use nc::poll;
use std::io::Read;
use std::ops::Deref;
use std::sync::Arc;
use wasmer::InstantiationError::{HostEnvInitialization, Link, Start};
use wasmer::{
    imports, Cranelift, Engine, Instance, Module, Store, Universal, UniversalEngine, Value,
};
use wasmer_wasi::WasiState;

#[derive(Clone)]
pub struct CompilationUnit {
    module: Module,
}

pub struct Compiler {
    pub engine: Arc<UniversalEngine>,
    store: Store,
}

impl Compiler {
    pub fn new() -> Compiler {
        let comp_config = Cranelift::default();
        let engine = Arc::new(Universal::new(comp_config).engine());
        let store = Store::new(engine.deref());
        Compiler { engine, store }
    }

    pub fn compile(&self, reader: &mut impl Read) -> Result<CompilationUnit, String> {
        let mut buff = vec![];
        reader
            .read_to_end(&mut buff)
            .expect("Cannot use reader during compilation");
        let module_result = Module::new(&self.store, buff);
        let compilation_unit = CompilationUnit {
            module: module_result.unwrap(),
        };
        let validation_error = get_validation_errors(&compilation_unit);
        if validation_error.is_some() {
            Err(validation_error.unwrap())
        } else {
            Ok(compilation_unit)
        }
    }
}

fn get_validation_errors(compilation_unit: &CompilationUnit) -> Option<String> {
    let instance = create_instance(&compilation_unit);
    if instance.is_ok() {
        let exports = instance.unwrap().exports;
        if exports.get_function(PROCESS_FN_SYM).is_err() {
            // TODO: check signature
            Some("Cannot find `process' function in executable".to_string())
        } else if exports.get_function(HANDLE_ERROR_FN_SYM).is_err() {
            Some("Cannot find `handle_error' in executable".to_string())
        } else {
            None
        }
    } else {
        None
    }
}

fn create_instance(compilation_unit: &CompilationUnit) -> Result<Instance, String> {
    let mut wasi_env = WasiState::new("runner").finalize();
    let import_object = wasi_env.unwrap().import_object(&compilation_unit.module);
    let instance = Instance::new(&compilation_unit.module, &import_object.unwrap());
    if instance.is_ok() {
        Ok(instance.unwrap())
    } else {
        match instance.unwrap_err() {
            Link(_error) => Err("Cannot create WASM instance: linking error".to_string()),
            Start(_error) => Err("Cannot create WASM instance: start error".to_string()),
            HostEnvInitialization(_error) => {
                Err("Cannot create WASM instance: host env init".to_string())
            }
            _ => Err("Cannot create WASM instance: unknown _error".to_string()),
        }
    }
}

pub struct Executor {
    pub engine: Arc<UniversalEngine>,
}

const PROCESS_FN_SYM: &str = "process";

const HANDLE_ERROR_FN_SYM: &str = "on_error";

impl Executor {
    pub fn new(&self, engine: Arc<UniversalEngine>) -> Executor {
        Executor { engine }
    }

    pub fn execute(
        &self,
        compilation_unit: CompilationUnit,
        frame: &DataFrame,
    ) -> Result<DataFrame, String> {
        match fork::fork() {
            Ok(Fork::Child) => {
                let instance_result = create_instance(&compilation_unit);
                if instance_result.is_ok() {
                    let instance = instance_result.unwrap();
                    let function_result = instance.exports.get_function(PROCESS_FN_SYM);
                    function_result.unwrap().call(&[]).unwrap();
                }
            }
            Ok(Fork::Parent(child)) => unsafe {
                let pidfd = nc::pidfd_open(child, 0);
                if pidfd == Err(nc::errno::ENOSYS) {
                    eprintln!("PIDFD_OPEN syscall not supported in this system: cannot execute runnable");
                } else {
                    let mut pollfd = libc::pollfd {
                        events: 0x001,
                        fd: pidfd.unwrap(),
                        revents: 0,
                    };
                    let r = libc::poll(&mut pollfd, 1, 2000);
                    if r >= 0 && pollfd.revents & libc::POLLIN == 0 {
                        let r = libc::kill(child, 0);
                        if r == 0 {
                            libc::kill(child, libc::SIGKILL);
                        }
                    }
                }
            },
            Err(_) => {
                println!()
            }
        }
        Ok(DataFrame {})
    }
}