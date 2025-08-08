use crate::log::*;
use crate::manifest::Manifest;
use clap::Args;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct CreateArgs {
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

impl CreateArgs {
    pub fn execute(&self) -> Result<(), String> {
        create_command(
            &self.manifest,
            &self.input_dir,
            &self.output_dir,
            self.verbose,
        )
    }
}

pub fn create_command(
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

    // Create all files referenced in the manifest
    let mut errors = Vec::new();

    // Process each storage device
    for (device_name, device) in &manifest.storage_devices {
        log_info(&format!("Processing storage device '{device_name}'."));

        // Copy fwup template file if device has fwup build args
        if let Some(build_args) = &device.build_args {
            if let Some(template) = build_args.fwup_template() {
                let input_path = input_dir.join(template);
                let output_path = output_dir.join(template);
                if let Err(e) = copy_file(&input_path, &output_path, verbose) {
                    errors.push(format!(
                        "Failed to copy fwup template '{template}' for device '{device_name}': {e}"
                    ));
                }
            }
        }

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

    // Copy the manifest file to the output directory
    if let Some(manifest_filename) = manifest_path.file_name() {
        let output_path = output_dir.join(manifest_filename);
        if let Err(e) = copy_file(manifest_path, &output_path, verbose) {
            errors.push(format!(
                "Failed to copy manifest file '{}': {e}",
                manifest_path.display()
            ));
        }
    }

    // Report errors
    if !errors.is_empty() {
        let mut error_msg = String::from("Create failed with the following errors:");
        for error in errors {
            error_msg.push_str(&format!("\n  - {error}"));
        }
        return Err(error_msg);
    }

    log_success("Created.");
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
    log_info(&format!("Processing image '{image_name}'."));

    // Copy fwup template file if image has fwup build args
    if let Some(build_args) = image.build_args() {
        if let Some(template) = build_args.fwup_template() {
            let input_path = input_dir.join(template);
            let output_path = output_dir.join(template);
            if let Err(e) = copy_file(&input_path, &output_path, verbose) {
                return Err(format!(
                    "Failed to copy fwup template '{template}' for image '{image_name}': {e}"
                ));
            }
        }
    }

    // If the image has files defined, copy those individual files
    let files = image.files();
    if !files.is_empty() {
        for file_entry in files {
            if let Err(e) = process_file_entry(file_entry, input_dir, output_dir, verbose) {
                return Err(format!(
                    "Failed to process file in image '{image_name}': {e}"
                ));
            }
        }
    }

    // Always copy the image file itself (regardless of whether it has individual files)
    let (input_filename, output_filename) = match image {
        crate::manifest::Image::String(filename) => (filename.as_str(), filename.as_str()),
        crate::manifest::Image::Object { out, .. } => (out.as_str(), out.as_str()),
    };

    let input_path = input_dir.join(input_filename);
    let output_path = output_dir.join(output_filename);

    copy_file(&input_path, &output_path, verbose)
}

fn process_file_entry(
    file_entry: &crate::manifest::FileEntry,
    input_dir: &Path,
    output_dir: &Path,
    verbose: bool,
) -> Result<(), String> {
    let input_filename = file_entry.input_filename();

    let input_path = input_dir.join(input_filename);
    let output_path = output_dir.join(input_filename);

    copy_file(&input_path, &output_path, verbose)
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
