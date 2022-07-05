use std::borrow::{Borrow, BorrowMut};
use std::collections::VecDeque;
use std::fmt::format;
use std::fs;
use crate::data::DataFrame;

use fork::Fork;
use std::io::{Read, SeekFrom, stderr, stdout};
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use wasi_cap_std_sync::file::File;
use std::io::Write;
use std::rc::Rc;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasmtime::{Module, Store, Engine, Instance, AsContextMut, Linker};
use wasmtime_wasi::WasiCtx;

#[derive(Clone)]
pub struct CompilationUnit {
    module: Module,
}

pub struct Compiler {
    engine: Arc<Engine>,
}

pub fn new_pair() -> (Compiler, Executor) {
    let mut config = wasmtime::Config::new();
    config.cache_config_load_default().expect("working cache directory");

    config.wasm_reference_types(false);
    config.wasm_simd(false);
    config.wasm_threads(false);
    config.wasm_bulk_memory(false);


    let engine_arc = Arc::new(wasmtime::Engine::new(&config).expect("WASM engine"));
    let compiler = Compiler::new(engine_arc.clone());
    let executor = Executor::new(engine_arc);
    (compiler, executor)
}

impl Compiler {
    pub fn new(engine: Arc<Engine>) -> Compiler {
        Compiler { engine }
    }

    pub fn compile(&self, reader: &mut impl Read) -> Result<CompilationUnit, String> {
        let mut buff = vec![];
        reader
            .read_to_end(&mut buff)
            .expect("Cannot use reader during compilation");
        match Module::new(&self.engine, buff) {
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
    /* match create_instance(compilation_unit, "".to_owned()) {
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
    } */
    None
}

pub struct Executor {
    engine: Arc<Engine>,
}

impl Executor {
    pub fn new(engine: Arc<Engine>) -> Executor {
        Executor { engine }
    }

    pub fn execute(
        &self,
        compilation_unit: &Option<CompilationUnit>,
        frame: &DataFrame,
    ) -> anyhow::Result<DataFrame> {
        let mut input_file = memfile::MemFile::create_default("tmp-stdin")?;
        let mut output_file = memfile::MemFile::create_default("tmp-stdout")?;
        let mut err_file = memfile::MemFile::create_default("tmp-stderr")?;

        input_file.write_all(&frame.body)?;

        let stdin = Box::new(wasi_common::pipe::ReadPipe::from_shared(Arc::new(RwLock::new(input_file))));
        let mut output_guarded = Arc::new(RwLock::new(output_file));
        let stderr = Box::new(wasi_common::pipe::WritePipe::from_shared(output_guarded.clone()));
        let mut stdout = Box::new(wasi_common::pipe::WritePipe::from_shared(Arc::new(RwLock::new(err_file))));

        let mut ctx = WasiCtxBuilder::new();
        let mut wasi_ctx = ctx
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr)
            .build();
        let mut store = Box::new(Store::new(&self.engine, wasi_ctx));
        let mut linker = Linker::new(&self.engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

        linker.module(store.as_context_mut(), "", &compilation_unit.as_ref().unwrap().module)?;
        linker.get_default(store.as_context_mut(), "")?
            .typed::<(), (), _>(store.as_context_mut())?
            .call(store.as_context_mut(), ())?;
        let mut buffer = vec![];
        let mut output = output_guarded.write().unwrap();
        output.read_to_end(&mut buffer)?;
        Ok(DataFrame {
            body: buffer
        })
    }
}
