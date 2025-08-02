use crate::commands::stone::Commands;
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
        Commands::Validate(args) => args.execute(),
        Commands::DescribeManifest(args) => args.execute(),
        Commands::Build(args) => args.execute(),
    }
}
