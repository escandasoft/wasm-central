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
    Compile {
        #[clap(short = 'I', long = "input_file")]
        input_file: std::path::PathBuf,

        #[clap(short = 'O', long = "output_file")]
        output_file: std::path::PathBuf,
    },
}
