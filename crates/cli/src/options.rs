use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long)]
    pub server_address_host: String,
    /// Name of the person to greet
    #[clap(short, long)]
    pub server_address_port: i16,

    /// Number of times to greet
    #[clap(subcommand)]
    pub command: Option<ModuleCommands>,
}

#[derive(Subcommand)]
pub enum ModuleCommands {
    List {},
    Load {
        #[clap(short, long)]
        file_path: std::path::PathBuf,
    },
    Compile {
        #[clap(short, long)]
        base: std::path::PathBuf,

        #[clap(short, long)]
        input_file: std::path::PathBuf,

        #[clap(short, long)]
        output_file: std::path::PathBuf,
    },
}