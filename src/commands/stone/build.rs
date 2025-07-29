use crate::fat::{FatImageOptions, FatType, create_fat_image};
use crate::log::*;
use crate::manifest::Manifest;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

pub fn build_command(
    manifest_path: PathBuf,
    input_dir: PathBuf,
    output_dir: PathBuf,
) -> Result<(), String> {
    // Check if manifest file exists
    if !manifest_path.exists() {
        return Err(format!(
            "Manifest file '{}' not found.",
            manifest_path.display()
        ));
    }

    let manifest = Manifest::from_file(&manifest_path)?;

    // Ensure output directory exists
    if let Err(e) = fs::create_dir_all(&output_dir) {
        return Err(format!(
            "Failed to create output directory '{}': {}",
            output_dir.display(),
            e
        ));
    }

    // Build all files referenced in the manifest
    let mut errors = Vec::new();

    // Process each storage device
    for (device_name, device) in &manifest.storage_devices {
        // Process each image in the device
        for (image_name, image) in &device.images {
            if let Err(e) = process_image(device_name, image_name, image, &input_dir, &output_dir) {
                errors.push(format!(
                    "Failed to process image '{}' in device '{}': {}",
                    image_name, device_name, e
                ));
            }
        }
    }

    // Report errors
    if !errors.is_empty() {
        let mut error_msg = String::from("Build failed with the following errors:");
        for error in errors {
            error_msg.push_str(&format!("\n  - {}", error));
        }
        return Err(error_msg);
    }

    log_success("Build completed successfully.");
    Ok(())
}

fn process_image(
    _device_name: &str,
    image_name: &str,
    image: &crate::manifest::Image,
    input_dir: &Path,
    output_dir: &Path,
) -> Result<(), String> {
    let input_path = input_dir.join(image.filename());
    let output_path = output_dir.join(image.filename());

    match image.build() {
        Some(build_type) => match build_type.as_str() {
            "mkfat" => build_mkfat(&input_path, &output_path, image),
            "mkfwup" => build_mkfwup(&input_path, &output_path, image),
            _ => Err(format!(
                "Unsupported build type '{}' for image '{}'.",
                build_type, image_name
            )),
        },
        None => {
            // Simple file copy
            copy_file(&input_path, &output_path)
        }
    }
}

fn copy_file(input_path: &Path, output_path: &Path) -> Result<(), String> {
    // Check if input file exists
    if !input_path.exists() {
        return Err(format!("Input file '{}' not found.", input_path.display()));
    }

    // Create output directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return Err(format!(
                "Failed to create output directory '{}': {}",
                parent.display(),
                e
            ));
        }
    }

    // Copy the file
    if let Err(e) = fs::copy(input_path, output_path) {
        return Err(format!(
            "Failed to copy file from '{}' to '{}': {}",
            input_path.display(),
            output_path.display(),
            e
        ));
    }

    log_success(&format!(
        "Copied file '{}' to '{}'.",
        input_path.display(),
        output_path.display()
    ));

    Ok(())
}

fn build_mkfat(
    input_path: &Path,
    output_path: &Path,
    image: &crate::manifest::Image,
) -> Result<(), String> {
    // Create output directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return Err(format!(
                "Failed to create output directory '{}': {}",
                parent.display(),
                e
            ));
        }
    }

    // Convert the manifest Image to a fat module compatible manifest
    let fat_manifest = create_fat_manifest(image)?;

    // Create a temporary manifest file for the fat module
    let temp_manifest_path = output_path.with_extension("manifest.json");
    let manifest_json = serde_json::to_string_pretty(&fat_manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

    fs::write(&temp_manifest_path, manifest_json)
        .map_err(|e| format!("Failed to write temporary manifest: {}", e))?;

    // Determine size from build_args or use default
    let size_mb = parse_size_from_args(image.build_args()).unwrap_or(32);

    // Create fat image options
    let options = FatImageOptions::new()
        .with_manifest_path(&temp_manifest_path)
        .with_base_path(input_path.parent().unwrap_or(Path::new(".")))
        .with_output_path(output_path)
        .with_size_mb(size_mb)
        .with_fat_type(FatType::Fat32)
        .with_verbose(false);

    // Create the fat image
    let result = create_fat_image(&options);

    // Clean up temporary manifest file
    let _ = fs::remove_file(&temp_manifest_path);

    match result {
        Ok(()) => {
            log_success(&format!("Created FAT image '{}'.", output_path.display()));
            Ok(())
        }
        Err(e) => Err(format!("Failed to create FAT image: {}", e)),
    }
}

#[derive(serde::Serialize)]
struct FatManifest {
    files: Vec<FatFileEntry>,
}

#[derive(serde::Serialize)]
struct FatFileEntry {
    filename: Option<String>,
    output: Option<String>,
}

fn create_fat_manifest(image: &crate::manifest::Image) -> Result<FatManifest, String> {
    let mut fat_files = Vec::new();

    for file_entry in image.files() {
        let fat_entry = match file_entry {
            crate::manifest::FileEntry::String(filename) => FatFileEntry {
                filename: Some(filename.clone()),
                output: None,
            },
            crate::manifest::FileEntry::Object { input, output } => FatFileEntry {
                filename: Some(input.clone()),
                output: Some(output.clone()),
            },
        };
        fat_files.push(fat_entry);
    }

    Ok(FatManifest { files: fat_files })
}

fn parse_size_from_args(args: &[String]) -> Option<u64> {
    for arg in args {
        if arg.starts_with("--size=") {
            if let Some(size_str) = arg.strip_prefix("--size=") {
                if let Ok(size) = size_str.parse::<u64>() {
                    return Some(size);
                }
            }
        }
    }
    None
}

fn build_mkfwup(
    input_path: &Path,
    output_path: &Path,
    _image: &crate::manifest::Image,
) -> Result<(), String> {
    // Create output directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return Err(format!(
                "Failed to create output directory '{}': {}",
                parent.display(),
                e
            ));
        }
    }

    // TODO: Implement mkfwup build logic
    // For now, this is a placeholder that will need actual mkfwup implementation
    log_error(&format!(
        "mkfwup build not yet implemented for image '{}'.",
        input_path.display()
    ));

    Err("mkfwup build functionality not yet implemented.".to_string())
}
