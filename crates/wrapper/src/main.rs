mod engine;

use quickjs_wasm_rs::{json, Context, Value};
use std::fs;

use once_cell::sync::OnceCell;
use std::io::{self, stderr, Read};

#[cfg(not(test))]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

static mut JS_CONTEXT: OnceCell<Context> = OnceCell::new();
static mut ENTRYPOINT: (OnceCell<Value>, OnceCell<Value>) = (OnceCell::new(), OnceCell::new());
static SCRIPT_NAME: &str = "script.js";

// TODO
//
// AOT validations:
//  1. Ensure that the required exports are present
//  2. If not present just evaluate the top level statement (?)

#[export_name = "wizer.initialize"]
pub extern "C" fn init() {
    unsafe {
        let mut context = Context::default();
        if context.register_globals(stderr(), stderr()).is_err() {
            eprintln!("Cannot register stderr as global for console and logger");
        }
        let mut contents = String::new();
        if io::stdin().read_to_string(&mut contents).is_err() {
            eprintln!("Cannot read stdin")
        } else {
            if let Err(err) = context.eval_global(SCRIPT_NAME, &contents) {
                eprintln!("Cannot eval script");
            } else {
                let global = context.global_object().unwrap();
                if let Ok(ns_object) = global.get_property("Namespace") {
                    if let Ok(main) = ns_object.get_property("main") {
                        JS_CONTEXT.set(context).unwrap();
                        ENTRYPOINT.0.set(ns_object).unwrap();
                        ENTRYPOINT.1.set(main).unwrap();
                    } else {
                        eprintln!("Cannot get main function callback");
                    }
                } else {
                    eprintln!("Cannot get Namespace object");
                }
            }
        }
    }
}

fn main() {
    unsafe {
        let context = JS_CONTEXT.get().unwrap();
        let receiver = ENTRYPOINT.0.get().unwrap();
        let main = ENTRYPOINT.1.get().unwrap();
        let input_bytes = engine::load().expect("Couldn't load input");

        let input_value = json::transcode_input(context, &input_bytes).unwrap();
        let output_value = main.call(receiver, &[input_value]);

        if output_value.is_err() {
            panic!("{}", output_value.unwrap_err().to_string());
        }

        let output = json::transcode_output(output_value.unwrap()).unwrap();
        engine::store(&output).expect("Couldn't store output");
    }
}
