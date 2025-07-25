use crate::commands::stone::Commands;
use crate::commands::stone::build::build_command;
use crate::commands::stone::describe_manifest::describe_manifest_command;

use crate::commands::stone::validate::validate_command;
use crate::log::*;
use clap::Parser;

mod commands;
mod fat;
mod log;
mod manifest;

#[derive(Parser, Debug)]
#[command(name = "stone")]
#[command(about = "A CLI for managing Avocado stones.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    if let Err(e) = run() {
        log_error(&e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate {
            manifest,
            input_dir,
        } => validate_command(manifest, input_dir),
        Commands::DescribeManifest { manifest } => describe_manifest_command(manifest),
        Commands::Build {
            manifest,
            input_dir,
            output_dir,
        } => build_command(manifest, input_dir, output_dir),
    }
}
