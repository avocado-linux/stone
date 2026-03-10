use clap::Subcommand;

pub mod bundle;
pub mod create;
pub mod describe_manifest;
pub mod provision;
pub mod validate;

use bundle::BundleArgs;
use create::CreateArgs;
use describe_manifest::DescribeManifestArgs;
use provision::ProvisionArgs;
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

    /// Build an OS bundle (.aos) containing all boot/OS artifacts for OTA and provisioning.
    Bundle(BundleArgs),

    /// Provision by actually building the artifacts specified in the manifest.
    Provision(ProvisionArgs),
}
