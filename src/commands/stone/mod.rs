use clap::Subcommand;

pub mod create;
pub mod describe_manifest;
pub mod validate;

use create::CreateArgs;
use describe_manifest::DescribeManifestArgs;
use validate::ValidateArgs;

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Describe the contents of a manifest file.
    #[command(name = "describe-manifest")]
    DescribeManifest(DescribeManifestArgs),

    /// Check if the manifest's inputs are satisfied.
    Validate(ValidateArgs),

    /// Create the artifacts specified in the manifest.
    Create(CreateArgs),
}
