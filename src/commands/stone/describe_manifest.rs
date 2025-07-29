use crate::log::*;
use crate::manifest::Manifest;
use std::path::PathBuf;

pub fn describe_manifest_command(manifest_path: PathBuf) -> Result<(), String> {
    // Check if manifest file exists
    if !manifest_path.exists() {
        return Err(format!(
            "Manifest file '{}' not found.",
            manifest_path.display()
        ));
    }

    let manifest = Manifest::from_file(&manifest_path)?;
    describe_manifest(&manifest);
    log_success("Described manifest.");
    Ok(())
}

// Helper function to convert size with units to a display string
fn format_size_display(size: Option<i64>, size_unit: Option<&String>) -> String {
    match (size, size_unit) {
        (Some(size_val), Some(unit)) => match unit.as_str() {
            "bytes" => format!("{} bytes", size_val),
            "kibibytes" => format!("{} KiB", size_val),
            "mebibytes" => format!("{} MiB", size_val),
            "gibibytes" => format!("{} GiB", size_val),
            "tebibytes" => format!("{} TiB", size_val),
            "kilobytes" => format!("{} KB", size_val),
            "megabytes" => format!("{} MB", size_val),
            "gigabytes" => format!("{} GB", size_val),
            "terabytes" => format!("{} TB", size_val),
            "blocks" => format!("{} blocks", size_val),
            _ => format!("{} {}", size_val, unit),
        },
        (Some(size_val), None) => format!("{}", size_val),
        _ => "-".to_string(),
    }
}

fn format_offset_display(offset: Option<i64>, offset_unit: Option<&String>) -> String {
    match (offset, offset_unit) {
        (Some(offset_val), Some(unit)) => match unit.as_str() {
            "bytes" => format!("{}", offset_val),
            "blocks" => format!("{}*blocks", offset_val),
            _ => format!("{} {}", offset_val, unit),
        },
        (Some(offset_val), None) => format!("{}", offset_val),
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
        output.push_str(&format!(
            "\nStorage Device: {}\n\
            ───────────────────────────────────────────────────────────────────────────────\n\
            Output File    : {}\n\
            Build Type     : {}\n",
            device_name, device.filename, device.build
        ));

        if let Some(build_conf) = &device.build_conf {
            output.push_str(&format!("Build Config   : {}\n", build_conf));
        }

        output.push_str(&format!("Device Path    : {}\n", device.devpath));

        if let Some(block_size) = device.block_size {
            output.push_str(&format!("Block Size     : {}\n", block_size));
        }

        // Images section
        output.push_str(&format!("\nImages ({} total):\n", device.images.len()));

        // Sort images for consistent output
        let mut images: Vec<_> = device.images.iter().collect();
        images.sort_by_key(|(name, _)| *name);

        for (image_name, image) in images {
            output.push_str(&format!("\n  • {} → {}\n", image_name, image.filename()));

            if let Some(build) = image.build() {
                output.push_str(&format!("    Build: {}\n", build));
            }

            if !image.files().is_empty() {
                output.push_str(&format!("    Files: {}\n", image.files().len()));
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

    println!("{}", output);
}
