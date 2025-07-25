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

// Note: Block sizes are assumed to be 512 bytes throughout this function.
// When converting from MB to blocks, we use: 1 MB = 2048 blocks (1024*1024 / 512).
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

        // Images section
        output.push_str(&format!("\nImages ({} total):\n", device.images.len()));

        // Sort images for consistent output
        let mut images: Vec<_> = device.images.iter().collect();
        images.sort_by_key(|(name, _)| *name);

        for (image_name, image) in images {
            output.push_str(&format!("\n  • {} → {}\n", image_name, image.filename));

            if let Some(build) = &image.build {
                output.push_str(&format!("    Build: {}\n", build));
            }

            if !image.files.is_empty() {
                output.push_str(&format!("    Files: {}\n", image.files.len()));
            }
        }

        // Partitions section
        output.push_str(&format!(
            "\nPartition Layout ({} partitions):\n",
            device.partitions.len()
        ));
        output.push_str("  #  Image        Offset    Size (blocks)  Special\n");
        output.push_str("  ─  ───────────  ────────  ─────────────  ────────────\n");

        for (idx, partition) in device.partitions.iter().enumerate() {
            let offset = partition.offset.as_deref().unwrap_or("-");
            let size = if let Some(blocks) = &partition.blocks {
                format!("{}", blocks)
            } else if let Some(size_mb) = &partition.size_mb {
                // Convert MB to blocks (1 MB = 2048 blocks, assuming 512-byte blocks)
                if let Ok(mb_value) = size_mb.parse::<u64>() {
                    format!("{}", mb_value * 2048)
                } else {
                    format!("{} (invalid MB value)", size_mb)
                }
            } else {
                "-".to_string()
            };
            let special = if partition.expand == Some("true".to_string()) {
                "expandable"
            } else {
                ""
            };

            output.push_str(&format!(
                "  {}  {:<11}  {:<8}  {:<13}  {}\n",
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
