use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Option<ModuleCommands>,
}

#[derive(Subcommand)]
pub enum ModuleCommands {
    List {
        #[clap(short = 'H', long = "host")]
        host: String,

        #[clap(short = 'p', long = "port")]
        port: i16,
    },
    Deploy {
        #[clap(short = 'H', long = "host")]
        host: String,

        #[clap(short = 'p', long = "port")]
        port: i16,

        #[clap(short = 'w', long = "wasm")]
        file_path: std::path::PathBuf,

        #[clap(short = 'i', long = "inputs")]
        inputs: String,

        #[clap(short = 'o', long = "outputs")]
        outputs: String,
    },
    Compile {
        #[clap(short, long)]
        input_file: std::path::PathBuf,

        #[clap(short, long)]
        output_file: std::path::PathBuf,
    },
}
