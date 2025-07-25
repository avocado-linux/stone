use clap::Subcommand;
use std::path::PathBuf;

pub mod build;
pub mod describe_manifest;

pub mod validate;

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Describe the contents of a manifest file.
    DescribeManifest {
        /// Path to the manifest.json file
        #[arg(
            short = 'm',
            long = "manifest",
            value_name = "FILE",
            default_value = "manifest.json"
        )]
        manifest: PathBuf,
    },
    /// Check if the manifest's inputs are satisfied.
    Validate {
        /// Path to the manifest.json file
        #[arg(
            short = 'm',
            long = "manifest-path",
            value_name = "FILE",
            default_value = "manifest.json"
        )]
        manifest: PathBuf,

        /// Path to the input directory
        #[arg(
            short = 'i',
            long = "input-dir",
            value_name = "DIR",
            default_value = "."
        )]
        input_dir: PathBuf,
    },
    Build {
        /// Path to the manifest.json file
        #[arg(
            short = 'm',
            long = "manifest-path",
            value_name = "FILE",
            default_value = "manifest.json"
        )]
        manifest: PathBuf,

        /// Path to the input directory
        #[arg(
            short = 'i',
            long = "input-dir",
            value_name = "DIR",
            default_value = "."
        )]
        input_dir: PathBuf,

        /// Path to the output directory
        #[arg(
            short = 'o',
            long = "output-dir",
            value_name = "DIR",
            default_value = "."
        )]
        output_dir: PathBuf,
    },
}
