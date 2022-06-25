use std::fs;
use std::io;
use std::path::PathBuf;
use std::rc::Rc;
use wasmtime::{Global, GlobalType, Mutability, Val, ValType};

use wizer::Wizer;

pub fn compile(base: &PathBuf, input_file: &PathBuf, output_file: &PathBuf) {
    let input_wasm = get_input_wasm_bytes();
    match Wizer::new()
        .make_linker(Some(Rc::new(|e: &wasmtime::Engine| {
            let mut linker = wasmtime::Linker::new(e);
            let ty = GlobalType::new(ValType::, Mutability::Const);
            let global = Global::new(&mut store, ty, Val::vec)?;
            linker.define("host", "offset", global)?;
            Ok(linker)
        })))
        .expect("Cannot create linker over WASM")
        .run(&input_wasm)
    {
        Ok(wasm_bytes) => {
            println!("Sucessfully compiled WASM with Wizer");
            match fs::File::create(output_file) {
                Ok(f) => f.write_all(&wasm_bytes),
                Err(err) => {
                    eprintln!("Couldn't write to output file at {}", output_file.display());
                }
            }
        }
        Err(err) => {
            eprintln!("Cannot compile WASM with Wizer {:?}", err);
        }
    }
    .expect("Couldn't compile");
}
