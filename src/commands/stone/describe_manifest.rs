use crate::log::*;
use crate::manifest::Manifest;
use clap::Args;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct DescribeManifestArgs {
    /// Path to the manifest.json file
    #[arg(
        short = 'm',
        long = "manifest-path",
        value_name = "PATH",
        default_value = "manifest.json"
    )]
    pub manifest: PathBuf,
}

impl DescribeManifestArgs {
    pub fn execute(&self) -> Result<(), String> {
        describe_manifest_command(&self.manifest)
    }
}

pub fn describe_manifest_command(manifest_path: &Path) -> Result<(), String> {
    // Check if manifest file exists
    if !manifest_path.exists() {
        return Err(format!(
            "Manifest file '{}' not found.",
            manifest_path.display()
        ));
    }

    let manifest = Manifest::from_file(manifest_path)?;
    describe_manifest(&manifest);
    log_success("Described manifest.");
    Ok(())
}

// Helper function to convert size with units to a display string
fn format_size_display(size: i64, size_unit: &str) -> String {
    match size_unit {
        "bytes" => format!("{size} bytes"),
        "kibibytes" => format!("{size} KiB"),
        "mebibytes" => format!("{size} MiB"),
        "gibibytes" => format!("{size} GiB"),
        "tebibytes" => format!("{size} TiB"),
        "kilobytes" => format!("{size} KB"),
        "megabytes" => format!("{size} MB"),
        "gigabytes" => format!("{size} GB"),
        "terabytes" => format!("{size} TB"),
        "blocks" => format!("{size} blocks"),
        _ => format!("{size} {size_unit}"),
    }
}

fn format_offset_display(offset: Option<i64>, offset_unit: Option<&String>) -> String {
    match (offset, offset_unit) {
        (Some(offset_val), Some(unit)) => match unit.as_str() {
            "bytes" => format!("{offset_val}"),
            "blocks" => format!("{offset_val}*blocks"),
            _ => format!("{offset_val} {unit}"),
        },
        (Some(offset_val), None) => format!("{offset_val}"),
        _ => "-".to_string(),
    }
}
fn describe_manifest(manifest: &Manifest) {
    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "Manifest Description\n\
        ═══════════════════════════════════════════════════════════════════════════════\n\
        Platform: {} ({})\n",
        manifest.runtime.platform, manifest.runtime.architecture
    ));

    // Storage devices
    for (device_name, device) in &manifest.storage_devices {
        let build_type = device
            .build_args
            .as_ref()
            .map(|args| args.build_type())
            .unwrap_or("none");

        output.push_str(&format!(
            "\nStorage Device: {}\n\
            ───────────────────────────────────────────────────────────────────────────────\n\
            Output File    : {}\n\
            Build Type     : {}\n",
            device_name, device.out, build_type
        ));

        output.push_str(&format!("Device Path    : {}\n", device.devpath));

        if let Some(block_size) = device.block_size {
            output.push_str(&format!("Block Size     : {block_size}\n"));
        }

        // Images section
        output.push_str(&format!("\nImages ({} total):\n", device.images.len()));

        // Sort images for consistent output
        let mut images: Vec<_> = device.images.iter().collect();
        images.sort_by_key(|(name, _)| *name);

        for (image_name, image) in images {
            output.push_str(&format!("\n  • {} → {}\n", image_name, image.out()));

            // Show size if this is an Object image
            if let Some(size) = image.size() {
                if let Some(size_unit) = image.size_unit() {
                    let size_display = format_size_display(size, size_unit);
                    output.push_str(&format!("    Size: {size_display}\n"));
                }
            }

            if let Some(build) = image.build() {
                output.push_str(&format!("    Build: {build}\n"));

                // Show build_args if present
                if let Some(build_args) = image.build_args() {
                    output.push_str("    Build Args:\n");
                    output.push_str(&format!("      type: {}\n", build_args.build_type()));
                    match build_args {
                        crate::manifest::BuildArgs::Fat { variant, files } => {
                            output.push_str(&format!("      variant: {variant:?}\n"));
                            if !files.is_empty() {
                                output.push_str(&format!("      files: {} file(s)\n", files.len()));
                            }
                        }
                        crate::manifest::BuildArgs::Fwup { template } => {
                            output.push_str(&format!("      template: \"{template}\"\n"));
                        }
                    }
                }
            }

            // Show files from build_args for fat builds, otherwise from image
            let files = if let Some(build_args) = image.build_args() {
                match build_args {
                    crate::manifest::BuildArgs::Fat { files, .. } => files.as_slice(),
                    _ => image.files(),
                }
            } else {
                image.files()
            };

            if !files.is_empty() {
                output.push_str(&format!("    Files ({}):\n", files.len()));
                for file_entry in files {
                    match file_entry {
                        crate::manifest::FileEntry::String(filename) => {
                            output.push_str(&format!("      {filename}\n"));
                        }
                        crate::manifest::FileEntry::Object {
                            input,
                            output: file_output,
                        } => {
                            output.push_str(&format!("      {input} → {file_output}\n"));
                        }
                    }
                }
            }
        }

        // Partitions section
        output.push_str(&format!(
            "\nPartition Layout ({} partitions):\n",
            device.partitions.len()
        ));
        output.push_str("  #  Image        Offset       Size           Special\n");
        output.push_str("  ─  ───────────  ───────────  ─────────────  ────────────\n");

        for (idx, partition) in device.partitions.iter().enumerate() {
            let offset = format_offset_display(partition.offset, partition.offset_unit.as_ref());
            let size = format_size_display(partition.size, &partition.size_unit);
            let special = if partition.expand == Some("true".to_string()) {
                "expandable"
            } else {
                ""
            };

            let image_display = partition.image.as_deref().unwrap_or("-");
            output.push_str(&format!(
                "  {}  {:<11}  {:<11}  {:<13}  {}\n",
                idx + 1,
                image_display,
                offset,
                size,
                special
            ));
        }

        // Show storage device build information
        if let Some(build_args) = &device.build_args {
            output.push_str("\nStorage Device Build Args:\n");
            output.push_str(&format!("  type: {}\n", build_args.build_type()));
            match build_args {
                crate::manifest::BuildArgs::Fat { variant, files } => {
                    output.push_str(&format!("  variant: {variant:?}\n"));
                    if !files.is_empty() {
                        output.push_str(&format!("  files: {} file(s)\n", files.len()));
                    }
                }
                crate::manifest::BuildArgs::Fwup { template } => {
                    output.push_str(&format!("  template: \"{template}\"\n"));
                }
            }
        }
    }

    output.push_str(
        "═══════════════════════════════════════════════════════════════════════════════",
    );

    println!("{output}");
}
