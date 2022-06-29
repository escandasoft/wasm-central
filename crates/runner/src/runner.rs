use crate::data::DataFrame;

use fork::Fork;
use std::io::Read;
use std::ops::Deref;
use std::sync::Arc;
use wasmer::InstantiationError::{HostEnvInitialization, Link, Start};
use wasmer::{Cranelift, Instance, Module, Store, Universal, UniversalEngine};
use wasmer_wasi::WasiState;

#[derive(Clone)]
pub struct CompilationUnit {
    module: Module,
}

pub struct Compiler {
    store: Store,
}

pub fn new_pair() -> (Compiler, Executor) {
    let comp_config = Cranelift::default();
    let engine_arc = Arc::new(Universal::new(comp_config).engine());
    let compiler = Compiler::new(engine_arc.clone());
    let executor = Executor::new(engine_arc);
    (compiler, executor)
}

impl Compiler {
    pub fn new(engine: Arc<UniversalEngine>) -> Compiler {
        let store = Store::new(engine.deref());
        Compiler { store }
    }

    pub fn compile(&self, reader: &mut impl Read) -> Result<CompilationUnit, String> {
        let mut buff = vec![];
        reader
            .read_to_end(&mut buff)
            .expect("Cannot use reader during compilation");
        match Module::new(&self.store, buff) {
            Ok(module) => {
                let compilation_unit = CompilationUnit { module };
                if let Some(validation_error) = get_validation_errors(&compilation_unit) {
                    Err(validation_error)
                } else {
                    Ok(compilation_unit)
                }
            }
            Err(error) => Err(format!("{:?}", error)),
        }
    }
}

fn get_validation_errors(compilation_unit: &CompilationUnit) -> Option<String> {
    match create_instance(compilation_unit) {
        Ok(instance) => {
            let exports = instance.exports;
            if exports.get_function(PROCESS_FN_SYM).is_err() {
                // TODO: check signature
                Some("Cannot find `process' function in executable".to_string())
            } else if exports.get_function(HANDLE_ERROR_FN_SYM).is_err() {
                Some("Cannot find `handle_error' in executable".to_string())
            } else {
                None
            };
            None
        }
        Err(error) => Some(format!("Cannot create instance because error: {}", error)),
    }
}

fn create_instance(compilation_unit: &CompilationUnit) -> Result<Instance, String> {
    let wasi_env = WasiState::new("runner").finalize();
    let import_object = wasi_env.unwrap().import_object(&compilation_unit.module);
    match Instance::new(&compilation_unit.module, &import_object.unwrap()) {
        Ok(instance) => Ok(instance),
        Err(error) => match error {
            Link(_error) => Err("Cannot create WASM instance: linking error".to_string()),
            Start(_error) => Err("Cannot create WASM instance: start error".to_string()),
            HostEnvInitialization(_error) => {
                Err("Cannot create WASM instance: host env init".to_string())
            }
            _ => Err("Cannot create WASM instance: unknown _error".to_string()),
        },
    }
}

pub struct Executor;

const PROCESS_FN_SYM: &str = "process";

const HANDLE_ERROR_FN_SYM: &str = "on_error";

impl Executor {
    pub fn new(_engine: Arc<UniversalEngine>) -> Executor {
        Executor {}
    }

    pub fn execute(
        &self,
        compilation_unit: &CompilationUnit,
        _frame: &DataFrame,
    ) -> Result<DataFrame, String> {
        match fork::fork() {
            Ok(Fork::Child) => match create_instance(compilation_unit) {
                Ok(instance) => match instance.exports.get_function(PROCESS_FN_SYM) {
                    Ok(exported_fn) => {
                        exported_fn.call(&[]).unwrap();
                    }
                    Err(error) => {
                        eprintln!(
                            "Cannot get exported function {}: {:?}",
                            PROCESS_FN_SYM, error
                        );
                    }
                },
                Err(error) => {
                    eprintln!(
                        "Cannot create instance of compilation unit because {}",
                        error
                    )
                }
            },
            Ok(Fork::Parent(child)) => unsafe {
                let pidfd = nc::pidfd_open(child, 0);
                if pidfd == Err(nc::errno::ENOSYS) {
                    eprintln!(
                        "PIDFD_OPEN syscall not supported in this system: cannot execute runnable"
                    );
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
