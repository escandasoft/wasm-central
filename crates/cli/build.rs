use std::env;
use std::fs;
use std::path::PathBuf;

fn stub_engine_for_clippy() {
    let engine_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("engine.wasm");

    if !engine_path.exists() {
        fs::write(engine_path, &[]).expect("failed to write empty engine.wasm stub");
        println!("cargo:warning=using stubbed engine.wasm for static analysis purposes...");
    }
}

fn copy_engine_binary() {
    let override_engine_path = env::var("WASM_ENGINE_PATH");
    let is_override = override_engine_path.is_ok();
    let mut engine_path = PathBuf::from(
        override_engine_path.unwrap_or_else(|_| env::var("CARGO_MANIFEST_DIR").unwrap()),
    );

    if !is_override {
        engine_path.pop();
        engine_path.pop();
        engine_path = engine_path.join("target/wasm32-wasi/release/wasm-central-wrapper.wasm");
    }

    println!("cargo:rerun-if-changed={:?}", engine_path);
    println!("cargo:rerun-if-changed=build.rs");

    if engine_path.exists() {
        let copied_engine_path =
            PathBuf::from(env::var("OUT_DIR").unwrap()).join("wasm-central-wrapper.wasm");

        fs::copy(&engine_path, &copied_engine_path).unwrap();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok("cargo-clippy") = env::var("CARGO_CFG_FEATURE").as_ref().map(String::as_str) {
        stub_engine_for_clippy();
    } else {
        copy_engine_binary();
    }
    Ok(())
}
