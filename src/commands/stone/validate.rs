use crate::log::*;
use crate::manifest::Manifest;
use std::path::PathBuf;

pub fn validate_command(manifest_path: PathBuf, input_dir: PathBuf) -> Result<(), String> {
    // Check if manifest file exists
    if !manifest_path.exists() {
        return Err(format!(
            "Manifest file '{}' not found.",
            manifest_path.display()
        ));
    }

    let manifest = Manifest::from_file(&manifest_path)?;

    // Validate all files referenced in the manifest
    let mut missing_files = Vec::new();

    // Process each storage device
    for (device_name, device) in &manifest.storage_devices {
        // Process each image in the device
        for (image_name, image) in &device.images {
            // Process each file in the image
            for file_entry in &image.files {
                let file_path = input_dir.join(&file_entry.filename);

                if !file_path.exists() {
                    missing_files.push((
                        device_name.clone(),
                        image_name.clone(),
                        file_entry.filename.clone(),
                    ));
                }
            }
        }
    }

    // Report results
    if !missing_files.is_empty() {
        let mut error_msg = format!(
            "Validation failed. {} file{} not found:",
            missing_files.len(),
            if missing_files.len() == 1 { "" } else { "s" }
        );

        for (device, image, filename) in missing_files {
            error_msg.push_str(&format!(
                "\n  - {} (device: {}, image: {})",
                filename, device, image
            ));
        }

        return Err(error_msg);
    }

    log_success("Validated.");
    Ok(())
}
