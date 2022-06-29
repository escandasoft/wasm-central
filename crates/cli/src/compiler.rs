use std::fs;
use std::io::{stderr, stdout};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use wizer::Wizer;

const WASM: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/wasm-central-wrapper.wasm"));

pub fn compile(input_file: &PathBuf, output_file: &PathBuf) -> () {
    match fs::File::open(input_file) {
        Ok(mut file) => unsafe {
            let mut file_buffer = vec![];
            file.read_to_end(&mut file_buffer)
                .expect("Cannot read to end");

            let stdin = wasi_common::pipe::ReadPipe::from(file_buffer);
            let stderr = wasi_common::pipe::WritePipe::from_shared(Arc::new(RwLock::new(stderr())));
            let stdout = wasi_common::pipe::WritePipe::from_shared(Arc::new(RwLock::new(stdout())));

            let mut wizer = Wizer::new();
            let mut wizer = wizer
                .allow_wasi(true)
                .expect("Cannot enable WASI")
                .inherit_stdio(true);
            let new_wasm = wizer
                .run(&WASM, Box::new(stdin), Box::new(stderr), Box::new(stdout))
                .expect("Cannot run Wizer");
            match fs::File::create(output_file) {
                Ok(mut o_file) => {
                    o_file
                        .write_all(&new_wasm.to_vec())
                        .expect("Cannot write output file with initialized WASM");
                    println!("Successfully compiled input file");
                }
                Err(_) => {
                    eprintln!("Cannot create output file at {}", output_file.display());
                }
            }
        },
        Err(_) => {
            eprintln!("Cannot open input file at {}", input_file.display());
        }
    }
    ()
}
