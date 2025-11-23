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

    /// Path to the input directory (can be specified multiple times for search priority)
    #[arg(
        short = 'i',
        long = "input-dir",
        value_name = "DIR",
        default_value = "."
    )]
    pub input_dirs: Vec<PathBuf>,
}

impl ValidateArgs {
    pub fn execute(&self) -> Result<(), String> {
        validate_command(&self.manifest, &self.input_dirs)
    }
}

/// Helper function to find a file in multiple input directories, searching in order
fn find_file_in_dirs(filename: &str, input_dirs: &[PathBuf]) -> Option<PathBuf> {
    for dir in input_dirs {
        let candidate = dir.join(filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub fn validate_command(manifest_path: &Path, input_dirs: &[PathBuf]) -> Result<(), String> {
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
    let mut missing_device_files = Vec::new();
    let mut missing_provision_files = Vec::new();

    // Check if provision file exists if specified in runtime
    if let Some(provision_file) = &manifest.runtime.provision
        && find_file_in_dirs(provision_file, input_dirs).is_none()
    {
        missing_files.push((
            "runtime".to_string(),
            "provision".to_string(),
            provision_file.clone(),
        ));
    }

    // Check provision profiles and their scripts
    if let Some(provision) = &manifest.provision {
        for (profile_name, profile) in &provision.profiles {
            if find_file_in_dirs(&profile.script, input_dirs).is_none() {
                missing_provision_files
                    .push((format!("Profile '{profile_name}'"), profile.script.clone()));
            }
        }

        // Check if provision_default references a valid profile
        if let Some(default_profile_name) = &manifest.runtime.provision_default
            && manifest
                .get_provision_profile(default_profile_name)
                .is_none()
        {
            missing_provision_files.push((
                "Default profile reference".to_string(),
                format!("Profile '{default_profile_name}' not found in provision.profiles"),
            ));
        }
        // Note: We don't check if the default profile's script exists here
        // because that will already be caught when validating all profiles above
    } else if manifest.runtime.provision_default.is_some() {
        missing_provision_files.push((
            "Default profile reference".to_string(),
            "provision_default specified but no provision section found".to_string(),
        ));
    }

    // Process each storage device
    for (device_name, device) in &manifest.storage_devices {
        // Check fwup template file if device has fwup build args
        if let Some(build_args) = &device.build_args
            && let Some(template) = build_args.fwup_template()
            && find_file_in_dirs(template, input_dirs).is_none()
        {
            missing_device_files.push((device_name.clone(), template.to_string()));
        }

        // Process each image in the device
        for (image_name, image) in &device.images {
            // Check if this is a string-type image (direct file reference)
            if let crate::manifest::Image::String(filename) = image
                && find_file_in_dirs(filename, input_dirs).is_none()
            {
                missing_files.push((device_name.clone(), image_name.clone(), filename.clone()));
            }

            // For fwup builds, check if template file exists
            if let Some(build_type) = image.build()
                && build_type == "fwup"
                && let Some(build_args) = image.build_args()
                && let Some(template) = build_args.fwup_template()
                && find_file_in_dirs(template, input_dirs).is_none()
            {
                missing_files.push((
                    device_name.clone(),
                    image_name.clone(),
                    template.to_string(),
                ));
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
                if find_file_in_dirs(file_entry.input_filename(), input_dirs).is_none() {
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
    if !missing_files.is_empty()
        || !missing_device_files.is_empty()
        || !missing_provision_files.is_empty()
    {
        let total_missing =
            missing_files.len() + missing_device_files.len() + missing_provision_files.len();
        let mut error_msg = format!("Validation failed. {total_missing} file(s) not found:");

        // Report missing provision files
        for (provision_type, filename) in missing_provision_files {
            error_msg.push_str(&format!("\n  provision: {provision_type}"));
            error_msg.push_str(&format!("\n    {filename}"));
        }

        // Report missing device-level files
        for (device, filename) in missing_device_files {
            error_msg.push_str(&format!("\n  device: {device}"));
            error_msg.push_str(&format!("\n    {filename}"));
        }

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
