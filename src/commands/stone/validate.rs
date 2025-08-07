use crate::log::*;
use crate::manifest::Manifest;
use clap::Args;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct ValidateArgs {
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
}

impl ValidateArgs {
    pub fn execute(&self) -> Result<(), String> {
        validate_command(&self.manifest, &self.input_dir)
    }
}

pub fn validate_command(manifest_path: &Path, input_dir: &Path) -> Result<(), String> {
    // Check if manifest file exists
    if !manifest_path.exists() {
        return Err(format!(
            "Manifest file '{}' not found.",
            manifest_path.display()
        ));
    }

    let manifest = Manifest::from_file(manifest_path)?;

    // Validate all files referenced in the manifest
    let mut missing_files = Vec::new();

    // Process each storage device
    for (device_name, device) in &manifest.storage_devices {
        // Process each image in the device
        for (image_name, image) in &device.images {
            // Check if this is a string-type image (direct file reference)
            if let crate::manifest::Image::String(filename) = image {
                let file_path = input_dir.join(filename);

                if !file_path.exists() {
                    missing_files.push((device_name.clone(), image_name.clone(), filename.clone()));
                }
            }

            // For fwup builds, check if template file exists
            if let Some(build_type) = image.build() {
                if build_type == "fwup" {
                    if let Some(build_args) = image.build_args() {
                        if let Some(template) = build_args.fwup_template() {
                            let template_path = input_dir.join(template);
                            if !template_path.exists() {
                                missing_files.push((
                                    device_name.clone(),
                                    image_name.clone(),
                                    template.to_string(),
                                ));
                            }
                        }
                    }
                }
            }

            // Validate build_args for different build types
            if let Some(build_type) = image.build() {
                if let Some(_build_args) = image.build_args() {
                    match build_type.as_str() {
                        "fat" => {
                            // Size is now required in the manifest structure
                        }
                        "fwup" => {
                            // Template is already checked above
                        }
                        _ => {}
                    }
                } else if build_type == "fat" || build_type == "fwup" {
                    // build_args is required for fat and fwup builds
                    missing_files.push((
                        device_name.clone(),
                        image_name.clone(),
                        format!("build_args (required for {build_type} builds)"),
                    ));
                }
            }

            // Process files from build_args for fat builds, otherwise from image
            let files = if let Some(build_args) = image.build_args() {
                match build_args {
                    crate::manifest::BuildArgs::Fat { files, .. } => files.as_slice(),
                    _ => image.files(),
                }
            } else {
                image.files()
            };

            for file_entry in files {
                let file_path = input_dir.join(file_entry.input_filename());

                if !file_path.exists() {
                    missing_files.push((
                        device_name.clone(),
                        image_name.clone(),
                        file_entry.input_filename().to_string(),
                    ));
                }
            }
        }
    }

    // Report results
    if !missing_files.is_empty() {
        let mut error_msg = format!(
            "Validation failed. {} file(s) not found:",
            missing_files.len()
        );

        // Group missing files by device and image
        let mut grouped: HashMap<(String, String), Vec<String>> = HashMap::new();
        for (device, image, filename) in missing_files {
            grouped.entry((device, image)).or_default().push(filename);
        }

        for ((device, image), filenames) in grouped {
            error_msg.push_str(&format!("\n  device: {device}, image: {image}"));
            for filename in filenames {
                error_msg.push_str(&format!("\n    {filename}"));
            }
        }

        return Err(error_msg);
    }

    log_success("Validated.");
    Ok(())
}
