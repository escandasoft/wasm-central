mod compiler;
mod options;

use clap::Parser;
use options::{Args, ModuleCommands};
use std::io::Read;
use std::{cmp, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if let Some(command) = args.command {
        match command {
            ModuleCommands::Compile {
                input_file,
                output_file,
            } => {
                compiler::compile(&input_file, &output_file);
            }
        }
    }
    Ok(())
}
