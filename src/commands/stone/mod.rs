use clap::Subcommand;

pub mod build;
pub mod describe_manifest;
pub mod validate;

use build::BuildArgs;
use describe_manifest::DescribeManifestArgs;
use validate::ValidateArgs;

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Describe the contents of a manifest file.
    #[command(name = "describe-manifest")]
    DescribeManifest(DescribeManifestArgs),

    /// Check if the manifest's inputs are satisfied.
    Validate(ValidateArgs),

    /// Build the artifacts specified in the manifest.
    Build(BuildArgs),
}
