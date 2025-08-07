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
fn format_size_display(size: Option<i64>, size_unit: Option<&String>) -> String {
    match (size, size_unit) {
        (Some(size_val), Some(unit)) => match unit.as_str() {
            "bytes" => format!("{size_val} bytes"),
            "kibibytes" => format!("{size_val} KiB"),
            "mebibytes" => format!("{size_val} MiB"),
            "gibibytes" => format!("{size_val} GiB"),
            "tebibytes" => format!("{size_val} TiB"),
            "kilobytes" => format!("{size_val} KB"),
            "megabytes" => format!("{size_val} MB"),
            "gigabytes" => format!("{size_val} GB"),
            "terabytes" => format!("{size_val} TB"),
            "blocks" => format!("{size_val} blocks"),
            _ => format!("{size_val} {unit}"),
        },
        (Some(size_val), None) => format!("{size_val}"),
        _ => "-".to_string(),
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
            .and_then(|args| args.get("type"))
            .and_then(|t| t.as_str())
            .unwrap_or("none");

        output.push_str(&format!(
            "\nStorage Device: {}\n\
            ───────────────────────────────────────────────────────────────────────────────\n\
            Output File    : {}\n\
            Build Type     : {}\n",
            device_name, device.out, build_type
        ));

        if let Some(build_args) = &device.build_args {
            output.push_str(&format!("Build Args     : {build_args:?}\n"));
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
            output.push_str(&format!("\n  • {} → {}\n", image_name, image.out()));

            if let Some(build) = image.build() {
                output.push_str(&format!("    Build: {build}\n"));

                // Show build_args if present
                if let Some(build_args) = image.build_args() {
                    output.push_str("    Build Args:\n");
                    for (key, value) in build_args {
                        output.push_str(&format!("      {key}: {value}\n"));
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
            let offset = format_offset_display(partition.offset, partition.offset_unit.as_ref());
            let size = format_size_display(partition.size, partition.size_unit.as_ref());
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
            for (key, value) in build_args {
                output.push_str(&format!("  {key}: {value}\n"));
            }
        }
    }

    output.push_str(
        "═══════════════════════════════════════════════════════════════════════════════",
    );

    println!("{output}");
}
