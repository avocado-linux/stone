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
    describe_manifest(&manifest)?;
    log_success("Described manifest.");
    Ok(())
}

// Helper function to convert size with units to a display string
fn format_size_display(size: Option<i64>, size_unit: Option<&String>) -> Result<String, String> {
    match (size, size_unit) {
        (Some(size_val), Some(unit)) => match unit.as_str() {
            "bytes" => Ok(format!("{size_val} bytes")),
            "kibibytes" => Ok(format!("{size_val} KiB")),
            "mebibytes" => Ok(format!("{size_val} MiB")),
            "gibibytes" => Ok(format!("{size_val} GiB")),
            "tebibytes" => Ok(format!("{size_val} TiB")),
            "kilobytes" => Ok(format!("{size_val} KB")),
            "megabytes" => Ok(format!("{size_val} MB")),
            "gigabytes" => Ok(format!("{size_val} GB")),
            "terabytes" => Ok(format!("{size_val} TB")),
            "blocks" => Ok(format!("{size_val} blocks")),
            _ => Err(format!(
                "Unknown size unit '{unit}'. Supported units: bytes, kibibytes, mebibytes, gibibytes, tebibytes, kilobytes, megabytes, gigabytes, terabytes, blocks."
            )),
        },
        (Some(size_val), None) => Ok(format!("{size_val}")),
        _ => Ok("-".to_string()),
    }
}

fn format_offset_display(
    offset: Option<i64>,
    offset_unit: Option<&String>,
) -> Result<String, String> {
    match (offset, offset_unit) {
        (Some(offset_val), Some(unit)) => match unit.as_str() {
            "bytes" => Ok(format!("{offset_val} bytes")),
            "blocks" => Ok(format!("{offset_val} blocks")),
            _ => Err(format!(
                "Unknown offset unit '{unit}'. Supported units: bytes, blocks."
            )),
        },
        (Some(offset_val), None) => Ok(format!("{offset_val}")),
        _ => Ok("-".to_string()),
    }
}
fn describe_manifest(manifest: &Manifest) -> Result<(), String> {
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

        if let Some(build_args) = &device.build_args {
            if let Some(variant) = build_args.variant() {
                output.push_str(&format!("Build Variant  : {variant}\n"));
            }

            if let Some(template) = build_args.template() {
                output.push_str(&format!("Build Template : {template}\n"));
            }
        }

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
            if image_name == image.out() {
                output.push_str(&format!("\n  • {}\n", image_name));
            } else {
                output.push_str(&format!("\n  • {} → {}\n", image_name, image.out()));
            }

            // Show size if present
            if let Some(size) = image.size() {
                let size_unit = match image {
                    crate::manifest::Image::String(_) => "megabytes",
                    crate::manifest::Image::Object { size_unit, .. } => {
                        size_unit.as_deref().unwrap_or("megabytes")
                    }
                };
                let size_display = format_size_display(Some(size), Some(&size_unit.to_string()))?;
                output.push_str(&format!("    Size: {size_display}\n"));
            }

            if let Some(_build) = image.build() {
                // Show build_args if present
                if let Some(build_args) = image.build_args() {
                    output.push_str("    Build Args:\n");
                    output.push_str(&format!("      type: {}\n", build_args.build_type()));

                    if let Some(variant) = build_args.variant() {
                        output.push_str(&format!("      variant: {variant}\n"));
                    }

                    if let Some(template) = build_args.template() {
                        output.push_str(&format!("      template: {template}\n"));
                    }

                    let files = build_args.files();
                    if !files.is_empty() {
                        output.push_str(&format!("      files: {} entries\n", files.len()));
                    }
                }
            }

            if !image.files().is_empty() {
                output.push_str(&format!("    Files ({}):\n", image.files().len()));
                for file_entry in image.files() {
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
            let offset = format_offset_display(partition.offset, partition.offset_unit.as_ref())?;
            let size = format_size_display(partition.size, partition.size_unit.as_ref())?;
            let special = if partition.expand == Some("true".to_string()) {
                "expandable"
            } else {
                ""
            };

            output.push_str(&format!(
                "  {}  {:<11}  {:<11}  {:<13}  {}\n",
                idx + 1,
                partition.image,
                offset,
                size,
                special
            ));
        }
    }

    output.push_str(
        "═══════════════════════════════════════════════════════════════════════════════",
    );

    println!("{output}");
    Ok(())
}
