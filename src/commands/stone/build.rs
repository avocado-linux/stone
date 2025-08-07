use crate::fat::{FatImageOptions, FatType, create_fat_image};
use crate::log::*;
use crate::manifest::Manifest;
use clap::Args;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct BuildArgs {
    /// Path to the manifest.json file
    #[arg(
        short = 'm',
        long = "manifest-path",
        value_name = "PATH",
        default_value = "manifest.json"
    )]
    pub manifest: PathBuf,

    /// Path to the input directory
    #[arg(
        short = 'i',
        long = "input-dir",
        value_name = "DIR",
        default_value = "."
    )]
    pub input_dir: PathBuf,

    /// Path to the output directory
    #[arg(
        short = 'o',
        long = "output-dir",
        value_name = "DIR",
        default_value = "."
    )]
    pub output_dir: PathBuf,

    /// Enable verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

impl BuildArgs {
    pub fn execute(&self) -> Result<(), String> {
        build_command(
            &self.manifest,
            &self.input_dir,
            &self.output_dir,
            self.verbose,
        )
    }
}

pub fn build_command(
    manifest_path: &Path,
    input_dir: &Path,
    output_dir: &PathBuf,
    verbose: bool,
) -> Result<(), String> {
    // Check if manifest file exists
    if !manifest_path.exists() {
        return Err(format!(
            "Manifest file '{}' not found.",
            manifest_path.display()
        ));
    }

    let manifest = Manifest::from_file(manifest_path)?;

    // Ensure output directory exists
    if let Err(e) = fs::create_dir_all(output_dir) {
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
        let build_type = device
            .build_args
            .as_ref()
            .map(|args| args.build_type())
            .unwrap_or("none");

        log_info(&format!(
            "Processing storage device '{device_name}' with build type '{build_type}'."
        ));

        // Process each image in the device
        for (image_name, image) in &device.images {
            if let Err(e) = process_image(
                device_name,
                image_name,
                image,
                input_dir,
                output_dir,
                verbose,
            ) {
                errors.push(format!(
                    "Failed to process image '{image_name}' in device '{device_name}': {e}"
                ));
            }
        }
    }

    // Report errors
    if !errors.is_empty() {
        let mut error_msg = String::from("Build failed with the following errors:");
        for error in errors {
            error_msg.push_str(&format!("\n  - {error}"));
        }
        return Err(error_msg);
    }

    log_success("Built.");
    Ok(())
}

fn process_image(
    _device_name: &str,
    image_name: &str,
    image: &crate::manifest::Image,
    input_dir: &Path,
    output_dir: &Path,
    verbose: bool,
) -> Result<(), String> {
    // Determine build type for logging
    let build_type_name = match image.build() {
        Some(build_type) => build_type.clone(),
        None => "copy".to_string(),
    };

    log_info(&format!(
        "Processing image '{image_name}' with build type '{build_type_name}'."
    ));

    // Handle both string and object image types
    let (input_filename, output_filename) = match image {
        crate::manifest::Image::String(filename) => (filename.as_str(), filename.as_str()),
        crate::manifest::Image::Object { out, .. } => (out.as_str(), out.as_str()),
    };

    let input_path = input_dir.join(input_filename);
    let output_path = output_dir.join(output_filename);

    match image.build() {
        Some(build_type) => match build_type.as_str() {
            "fat" => build_fat(&input_path, &output_path, image, verbose),
            "fwup" => build_fwup(&input_path, &output_path, image, verbose),
            _ => Err(format!(
                "Unsupported build type '{build_type}' for image '{image_name}'."
            )),
        },
        None => {
            // Simple file copy
            copy_file(&input_path, &output_path, verbose)
        }
    }
}

fn copy_file(input_path: &Path, output_path: &Path, verbose: bool) -> Result<(), String> {
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

    if verbose {
        log_debug(&format!(
            "Copied from and to:\n  {}\n  {}",
            input_path.display(),
            output_path.display()
        ));
    }

    Ok(())
}

fn build_fat(
    input_path: &Path,
    output_path: &Path,
    image: &crate::manifest::Image,
    verbose: bool,
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
        .map_err(|e| format!("Failed to serialize manifest: {e}"))?;

    fs::write(&temp_manifest_path, manifest_json)
        .map_err(|e| format!("Failed to write temporary manifest: {e}"))?;

    // Extract required size from image
    let size_mb = extract_size_from_image(image)?;

    // Create fat image options
    let options = FatImageOptions::new()
        .with_manifest_path(&temp_manifest_path)
        .with_base_path(input_path.parent().unwrap_or(Path::new(".")))
        .with_output_path(output_path)
        .with_size_mb(size_mb)
        .with_fat_type(FatType::Fat32)
        .with_verbose(verbose);

    // Create the fat image
    let result = create_fat_image(&options);

    // Clean up temporary manifest file
    let _ = fs::remove_file(&temp_manifest_path);

    match result {
        Ok(()) => {
            if verbose {
                log_debug(&format!("Created FAT image '{}'.", output_path.display()));
            }
            Ok(())
        }
        Err(e) => Err(format!("Failed to create FAT image: {e}")),
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

    // Get files from build_args for fat builds
    let files = if let Some(build_args) = image.build_args() {
        build_args.fat_files()
    } else {
        &[]
    };

    for file_entry in files {
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

fn extract_size_from_image(image: &crate::manifest::Image) -> Result<u64, String> {
    match image {
        crate::manifest::Image::String(_) => Err(
            "FAT image build requires an object-type image with size specification.".to_string(),
        ),
        crate::manifest::Image::Object {
            size, size_unit, ..
        } => {
            let size_num =
                u64::try_from(*size).map_err(|_| format!("Invalid size value: {size}"))?;

            match size_unit.as_str() {
                "bytes" => Ok(size_num / (1024 * 1024)),
                "kibibytes" => Ok(size_num / 1024),
                "mebibytes" => Ok(size_num),
                "gibibytes" => Ok(size_num * 1024),
                "tebibytes" => Ok(size_num * 1024 * 1024),
                "kilobytes" => Ok((size_num * 1000) / (1024 * 1024)),
                "megabytes" => Ok((size_num * 1000 * 1000) / (1024 * 1024)),
                "gigabytes" => Ok((size_num * 1000 * 1000 * 1000) / (1024 * 1024)),
                "terabytes" => Ok((size_num * 1000 * 1000 * 1000 * 1000) / (1024 * 1024)),
                _ => Err(format!("Unsupported size unit: '{size_unit}'")),
            }
        }
    }
}

fn build_fwup(
    input_path: &Path,
    output_path: &Path,
    image: &crate::manifest::Image,
    verbose: bool,
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

    // Extract template from build_args
    let template = image
        .build_args()
        .and_then(|args| args.fwup_template())
        .ok_or_else(|| "fwup build requires BuildArgs::Fwup with template.".to_string())?;

    let template_path = input_path.join(template);

    // Check if template file exists
    if !template_path.exists() {
        return Err(format!(
            "[ERROR] Template file '{}' not found.",
            template_path.display()
        ));
    }

    // For now, implement a basic fwup call
    // This assumes fwup is installed on the system
    if verbose {
        log_debug(&format!(
            "Executing fwup command: fwup -c -f {} -o {}",
            template_path.display(),
            output_path.display()
        ));
    }

    let status = std::process::Command::new("fwup")
        .arg("-c")
        .arg("-f")
        .arg(&template_path)
        .arg("-o")
        .arg(output_path)
        .current_dir(input_path)
        .status();

    match status {
        Ok(exit_status) => {
            if exit_status.success() {
                log_success(&format!(
                    "Created fwup image '{}' using template '{}'.",
                    output_path.display(),
                    template
                ));
                Ok(())
            } else {
                Err(format!(
                    "fwup command failed with exit code: {}",
                    exit_status.code().unwrap_or(-1)
                ))
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(
                    "[ERROR] fwup command not found. Please install fwup to build firmware images."
                        .to_string(),
                )
            } else {
                Err(format!("Failed to execute fwup command: {e}"))
            }
        }
    }
}
