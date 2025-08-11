use crate::fat;
use crate::log::*;
use crate::manifest::{BuildArgs, FatVariant, FileEntry, Image, Manifest};
use clap::Args;

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Args, Debug)]
pub struct ProvisionArgs {
    /// Path to the input directory containing manifest.json
    #[arg(
        short = 'i',
        long = "input-dir",
        value_name = "DIR",
        default_value = "."
    )]
    pub input_dir: PathBuf,

    /// Enable verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

impl ProvisionArgs {
    pub fn execute(&self) -> Result<(), String> {
        provision_command(&self.input_dir, self.verbose)
    }
}

pub fn provision_command(input_dir: &Path, verbose: bool) -> Result<(), String> {
    // Find manifest.json in the input directory
    let manifest_path = input_dir.join("manifest.json");
    if !manifest_path.exists() {
        return Err(format!(
            "Manifest file 'manifest.json' not found in input directory '{}'.",
            input_dir.display()
        ));
    }
    let manifest = Manifest::from_file(&manifest_path)?;

    if verbose {
        log_info(&format!("Found manifest in '{}'.", input_dir.display()));
    }

    // Create _build directory for our work
    let build_dir = input_dir.join("_build");
    if let Err(e) = fs::create_dir_all(&build_dir) {
        return Err(format!(
            "Failed to create build directory '{}': {}",
            build_dir.display(),
            e
        ));
    }

    if verbose {
        log_info(&format!("Using build directory '{}'.", build_dir.display()));
    }

    // Process each storage device
    for (device_name, device) in &manifest.storage_devices {
        log_info(&format!("Provisioning storage device '{device_name}'."));

        // First, build all images in the device (inner dependencies)
        for (image_name, image) in &device.images {
            build_image(
                device_name,
                image_name,
                image,
                input_dir,
                &build_dir,
                verbose,
            )?;
        }

        // Then, build storage device if it has fwup build args (outer dependencies)
        if let Some(build_args) = &device.build_args {
            build_storage_device(
                device_name,
                device,
                build_args,
                &manifest,
                input_dir,
                &build_dir,
                verbose,
            )?;
        }
    }

    // Execute provision script if specified in runtime
    if let Some(provision_file) = &manifest.runtime.provision {
        execute_provision_script(provision_file, input_dir, &build_dir, verbose)?;
    }

    log_success("Provision completed.");
    Ok(())
}

fn build_storage_device(
    device_name: &str,
    device: &crate::manifest::StorageDevice,
    build_args: &BuildArgs,
    manifest: &Manifest,
    input_dir: &Path,
    build_dir: &Path,
    verbose: bool,
) -> Result<(), String> {
    match build_args {
        BuildArgs::Fwup { template } => {
            log_info(&format!(
                "Building storage device '{device_name}' with fwup template '{template}'."
            ));

            build_fwup_with_env_vars(
                device_name,
                device,
                template,
                manifest,
                input_dir,
                build_dir,
                verbose,
            )?;
        }
        BuildArgs::Fat { .. } => {
            return Err("FAT build args not supported for storage devices".to_string());
        }
    }

    Ok(())
}

fn build_image(
    device_name: &str,
    image_name: &str,
    image: &Image,
    input_dir: &Path,
    build_dir: &Path,
    verbose: bool,
) -> Result<(), String> {
    log_info(&format!(
        "Building image '{image_name}' in device '{device_name}'."
    ));

    match image {
        Image::String(_) => {
            // Simple string images don't need building, they're just file references
            if verbose {
                log_debug(&format!(
                    "Image '{image_name}' is a simple file reference, no build needed."
                ));
            }
            Ok(())
        }
        Image::Object {
            out,
            build_args: Some(build_args),
            size,
            size_unit,
            ..
        } => match build_args {
            BuildArgs::Fat { variant, files } => build_fat_image(FatImageParams {
                image_name,
                out,
                variant,
                files,
                size,
                size_unit,
                input_dir,
                build_dir,
                verbose,
            }),
            BuildArgs::Fwup { template } => {
                build_fwup_image(image_name, image, template, input_dir, build_dir, verbose)
            }
        },
        Image::Object {
            build_args: None, ..
        } => {
            // Object without build args doesn't need building
            if verbose {
                log_debug(&format!(
                    "Image '{image_name}' has no build args, no build needed."
                ));
            }
            Ok(())
        }
    }
}

struct FatImageParams<'a> {
    image_name: &'a str,
    out: &'a str,
    variant: &'a FatVariant,
    files: &'a [FileEntry],
    size: &'a i64,
    size_unit: &'a str,
    input_dir: &'a Path,
    build_dir: &'a Path,
    verbose: bool,
}

fn build_fat_image(params: FatImageParams) -> Result<(), String> {
    log_info(&format!(
        "Building FAT image '{}' -> '{}'.",
        params.image_name, params.out
    ));

    // Convert size to MB
    let size_mb = convert_size_to_mb(*params.size, params.size_unit)?;

    // Convert FatVariant to fat::FatType
    let fat_type = match params.variant {
        FatVariant::Fat12 => fat::FatType::Fat12,
        FatVariant::Fat16 => fat::FatType::Fat16,
        FatVariant::Fat32 => fat::FatType::Fat32,
    };

    // Create a temporary manifest for the FAT builder
    let fat_manifest = create_fat_manifest(params.files)?;
    let temp_manifest_path = params
        .build_dir
        .join(format!("temp_manifest_{}.json", params.image_name));

    // Write temporary manifest
    let manifest_json = serde_json::to_string_pretty(&fat_manifest)
        .map_err(|e| format!("Failed to serialize FAT manifest: {e}"))?;
    fs::write(&temp_manifest_path, manifest_json)
        .map_err(|e| format!("Failed to write temporary manifest: {e}"))?;

    let output_path = params.build_dir.join(params.out);

    // Create FAT image options
    let options = fat::FatImageOptions::new()
        .with_manifest_path(&temp_manifest_path)
        .with_base_path(params.input_dir)
        .with_output_path(&output_path)
        .with_size_mebibytes(size_mb)
        .with_fat_type(fat_type)
        .with_verbose(params.verbose);

    // Build the FAT image
    let result = fat::create_fat_image(&options);

    // Clean up temporary manifest
    let _ = fs::remove_file(&temp_manifest_path);

    result?;

    log_success(&format!("Built FAT image '{}'.", params.out));
    Ok(())
}

fn build_fwup_image(
    image_name: &str,
    image: &Image,
    template: &str,
    input_dir: &Path,
    build_dir: &Path,
    verbose: bool,
) -> Result<(), String> {
    let out = image.out();
    log_info(&format!(
        "Building fwup image '{image_name}' -> '{out}' using template '{template}'."
    ));

    let template_path = input_dir.join(template);
    let output_path = build_dir.join(out);

    let mut cmd = Command::new("fwup");
    cmd.arg("-c")
        .arg("-f")
        .arg(&template_path)
        .arg("-o")
        .arg(&output_path)
        .current_dir(build_dir);

    // Set disk-specific environment variables if present
    if let Some(block_size) = image.block_size() {
        cmd.env("AVOCADO_DISK_BLOCK_SIZE", block_size.to_string());
    }
    if let Some(uuid) = image.uuid() {
        cmd.env("AVOCADO_DISK_UUID", uuid);
    }

    if verbose {
        log_debug(&format!(
            "Executing fwup in '{}': fwup -c -f {} -o {}",
            build_dir.display(),
            template_path.display(),
            output_path.display()
        ));

        // Log disk-specific environment variables
        if let Some(block_size) = image.block_size() {
            log_debug(&format!("  AVOCADO_DISK_BLOCK_SIZE={block_size}"));
        }
        if let Some(uuid) = image.uuid() {
            log_debug(&format!("  AVOCADO_DISK_UUID={uuid}"));
        }
    }

    let status = cmd.status().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "fwup command not found. Please install fwup to build firmware packages.".to_string()
        } else {
            format!("Failed to execute fwup command: {e}")
        }
    })?;

    if !status.success() {
        return Err(format!(
            "fwup command failed with exit code: {}",
            status.code().unwrap_or(-1)
        ));
    }

    log_success(&format!("Built fwup image '{out}'."));
    Ok(())
}

fn convert_size_to_mb(size: i64, size_unit: &str) -> Result<u64, String> {
    let size_mb = match size_unit.to_lowercase().as_str() {
        "bytes" | "byte" | "b" => size as f64 / (1024.0 * 1024.0),
        "kilobytes" | "kilobyte" | "kb" => size as f64 / 1024.0,
        "kibibytes" | "kibibyte" | "kib" => size as f64 / 1024.0,
        "megabytes" | "megabyte" | "mb" => size as f64,
        "mebibytes" | "mebibyte" | "mib" => size as f64,
        "gigabytes" | "gigabyte" | "gb" => size as f64 * 1024.0,
        "gibibytes" | "gibibyte" | "gib" => size as f64 * 1024.0,
        _ => {
            return Err(format!("Unsupported size unit: {size_unit}"));
        }
    };

    if size_mb <= 0.0 {
        return Err("Image size must be positive".to_string());
    }

    Ok(size_mb.ceil() as u64)
}

fn create_fat_manifest(files: &[FileEntry]) -> Result<fat::Manifest, String> {
    let fat_files: Vec<fat::FileEntry> = files
        .iter()
        .map(|entry| match entry {
            FileEntry::String(filename) => fat::FileEntry {
                filename: Some(filename.clone()),
                output: None,
            },
            FileEntry::Object { input, output } => fat::FileEntry {
                filename: Some(input.clone()),
                output: Some(output.clone()),
            },
        })
        .collect();

    Ok(fat::Manifest {
        files: fat_files,
        directories: None,
    })
}

fn build_fwup_with_env_vars(
    device_name: &str,
    device: &crate::manifest::StorageDevice,
    template: &str,
    manifest: &Manifest,
    input_dir: &Path,
    build_dir: &Path,
    verbose: bool,
) -> Result<(), String> {
    let template_path = input_dir.join(template);
    let output_path = build_dir.join(&device.out);

    // Calculate environment variables from manifest
    let env_vars = calculate_avocado_env_vars(device_name, device, manifest, input_dir, build_dir)?;

    let mut cmd = Command::new("fwup");
    cmd.arg("-c")
        .arg("-f")
        .arg(&template_path)
        .arg("-o")
        .arg(&output_path)
        .current_dir(build_dir);

    // Set all AVOCADO environment variables
    for (key, value) in &env_vars {
        cmd.env(key, value);
    }

    if verbose {
        log_debug(&format!(
            "Executing fwup in '{}': fwup -c -f {} -o {}",
            build_dir.display(),
            template_path.display(),
            output_path.display()
        ));
        log_debug("Environment variables:");
        for (key, value) in &env_vars {
            log_debug(&format!("  {key}={value}"));
        }
    }

    let status = cmd.status().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "fwup command not found. Please install fwup to build firmware packages.".to_string()
        } else {
            format!("Failed to execute fwup command: {e}")
        }
    })?;

    if !status.success() {
        // Show environment variables when fwup fails to help with debugging
        if !verbose {
            log_debug("Environment variables used:");
            for (key, value) in &env_vars {
                log_debug(&format!("  {key}={value}"));
            }
        }
        return Err(format!(
            "fwup command failed with exit code: {}",
            status.code().unwrap_or(-1)
        ));
    }

    log_success(&format!(
        "Created firmware package '{}' using configuration '{}'.",
        output_path.display(),
        template_path.display()
    ));

    Ok(())
}

fn calculate_avocado_env_vars(
    _device_name: &str,
    device: &crate::manifest::StorageDevice,
    manifest: &Manifest,
    input_dir: &Path,
    build_dir: &Path,
) -> Result<HashMap<String, String>, String> {
    let mut env_vars = HashMap::new();

    // No longer setting AVOCADO_SDK_RUNTIME_DIR - image paths are now absolute

    // Meta Data - read from os-release file and manifest
    let (os_version, os_codename, os_description, os_author) = read_os_release_info(input_dir)?;
    env_vars.insert("AVOCADO_OS_VERSION".to_string(), os_version);
    env_vars.insert("AVOCADO_OS_CODENAME".to_string(), os_codename);
    env_vars.insert("AVOCADO_OS_DESCRIPTION".to_string(), os_description);
    env_vars.insert("AVOCADO_OS_AUTHOR".to_string(), os_author);

    // Read platform and architecture from manifest runtime section
    env_vars.insert(
        "AVOCADO_OS_PLATFORM".to_string(),
        manifest.runtime.platform.clone(),
    );
    env_vars.insert(
        "AVOCADO_OS_ARCHITECTURE".to_string(),
        manifest.runtime.architecture.clone(),
    );

    // Device Info
    let block_size = device.block_size.unwrap_or(512);

    // Set disk-specific environment variables if present on storage device
    if let Some(device_block_size) = device.block_size {
        env_vars.insert(
            "AVOCADO_DISK_BLOCK_SIZE".to_string(),
            device_block_size.to_string(),
        );
    }
    if let Some(device_uuid) = &device.uuid {
        env_vars.insert("AVOCADO_DISK_UUID".to_string(), device_uuid.clone());
    }

    // Dynamically set image environment variables with full paths
    for (image_name, image) in &device.images {
        let name_upper = image_name.to_uppercase();
        let env_var_name = format!("AVOCADO_IMAGE_{name_upper}");

        // Determine the full path based on image type
        let image_path = match image {
            Image::String(filename) => {
                // Input files are in the input directory
                input_dir.join(filename).to_string_lossy().to_string()
            }
            Image::Object {
                out,
                build_args: Some(_),
                ..
            } => {
                // Generated files (with build_args) are in the build directory
                build_dir.join(out).to_string_lossy().to_string()
            }
            Image::Object {
                out,
                build_args: None,
                ..
            } => {
                // Object files without build_args are input files in the input directory
                input_dir.join(out).to_string_lossy().to_string()
            }
        };

        env_vars.insert(env_var_name, image_path);
    }

    // Calculate partition offsets and sizes from the partition table
    let mut current_offset = 0u64;

    for partition in &device.partitions {
        let partition_offset = if let Some(offset) = partition.offset {
            convert_to_blocks(
                offset,
                partition.offset_unit.as_deref().unwrap_or("blocks"),
                block_size,
            )?
        } else {
            current_offset
        };

        let partition_size = convert_to_blocks(partition.size, &partition.size_unit, block_size)?;

        // Set partition variables based on the partition name
        if let Some(partition_name) = &partition.name {
            let name_upper = partition_name.to_uppercase().replace(['-', ' '], "_");

            // Set offset for this partition
            env_vars.insert(
                format!("AVOCADO_PARTITION_{name_upper}_OFFSET"),
                partition_offset.to_string(),
            );

            // Set size in blocks for this partition
            env_vars.insert(
                format!("AVOCADO_PARTITION_{name_upper}_BLOCKS"),
                partition_size.to_string(),
            );

            // Set redundant offset if present
            if let Some(offset_redundant) = partition.offset_redundant {
                let redundant_offset = convert_to_blocks(
                    offset_redundant,
                    partition
                        .offset_redundant_unit
                        .as_deref()
                        .unwrap_or("blocks"),
                    block_size,
                )?;
                env_vars.insert(
                    format!("AVOCADO_PARTITION_{name_upper}_OFFSET_REDUND"),
                    redundant_offset.to_string(),
                );
            }

            // Set expand property if present
            if let Some(expand) = &partition.expand {
                env_vars.insert(
                    format!("AVOCADO_PARTITION_{name_upper}_EXPAND"),
                    expand.to_string(),
                );
            }
        }

        current_offset = partition_offset + partition_size;
    }

    Ok(env_vars)
}

fn convert_to_blocks(size: i64, unit: &str, block_size: u32) -> Result<u64, String> {
    let bytes = match unit.to_lowercase().as_str() {
        "bytes" | "byte" | "b" => size as u64,
        "blocks" | "block" => return Ok(size as u64),
        "kilobytes" | "kilobyte" | "kb" => (size as u64) * 1024,
        "kibibytes" | "kibibyte" | "kib" => (size as u64) * 1024,
        "megabytes" | "megabyte" | "mb" => (size as u64) * 1024 * 1024,
        "mebibytes" | "mebibyte" | "mib" => (size as u64) * 1024 * 1024,
        "gigabytes" | "gigabyte" | "gb" => (size as u64) * 1024 * 1024 * 1024,
        "gibibytes" | "gibibyte" | "gib" => (size as u64) * 1024 * 1024 * 1024,
        _ => return Err(format!("Unsupported size unit: {unit}")),
    };

    Ok(bytes / (block_size as u64))
}

fn read_os_release_info(input_dir: &Path) -> Result<(String, String, String, String), String> {
    let os_release_path = input_dir.join("os-release");

    if !os_release_path.exists() {
        return Err(format!(
            "OS release file 'os-release' not found in input directory '{}'.",
            input_dir.display()
        ));
    }

    let content = fs::read_to_string(&os_release_path).map_err(|e| {
        format!(
            "Failed to read os-release file '{}': {}",
            os_release_path.display(),
            e
        )
    })?;

    let mut version_id = None;
    let mut version_codename = None;
    let mut pretty_name = None;
    let mut vendor_name = None;

    // Parse the os-release file to find VERSION_ID, VERSION_CODENAME, PRETTY_NAME, and VENDOR_NAME
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("VERSION_ID=") {
            let version = line.strip_prefix("VERSION_ID=").unwrap();
            // Remove quotes if present
            let version = version.trim_matches('"').trim_matches('\'');
            version_id = Some(version.to_string());
        } else if line.starts_with("VERSION_CODENAME=") {
            let codename = line.strip_prefix("VERSION_CODENAME=").unwrap();
            // Remove quotes if present
            let codename = codename.trim_matches('"').trim_matches('\'');
            version_codename = Some(codename.to_string());
        } else if line.starts_with("PRETTY_NAME=") {
            let name = line.strip_prefix("PRETTY_NAME=").unwrap();
            // Remove quotes if present
            let name = name.trim_matches('"').trim_matches('\'');
            pretty_name = Some(name.to_string());
        } else if line.starts_with("VENDOR_NAME=") {
            let vendor = line.strip_prefix("VENDOR_NAME=").unwrap();
            // Remove quotes if present
            let vendor = vendor.trim_matches('"').trim_matches('\'');
            vendor_name = Some(vendor.to_string());
        }
    }

    let version = version_id.ok_or_else(|| {
        format!(
            "VERSION_ID field not found in os-release file '{}'.",
            os_release_path.display()
        )
    })?;

    let codename = version_codename.unwrap_or_else(String::new);
    let description = pretty_name.unwrap_or_else(String::new);
    let author = vendor_name.unwrap_or_else(String::new);

    Ok((version, codename, description, author))
}

fn execute_provision_script(
    provision_file: &str,
    input_dir: &Path,
    build_dir: &Path,
    verbose: bool,
) -> Result<(), String> {
    let provision_path = input_dir.join(provision_file);

    if !provision_path.exists() {
        return Err(format!(
            "Provision file '{provision_file}' not found in input directory."
        ));
    }

    log_info(&format!(
        "Executing provision script '{}'.",
        provision_path.display()
    ));

    let mut command = Command::new(&provision_path);
    command.current_dir(input_dir);

    // Set environment variables for the provision script
    let manifest_path = input_dir.join("manifest.json");
    command.env("AVOCADO_STONE_MANIFEST", manifest_path);
    command.env("AVOCADO_STONE_BUILD_DIR", build_dir);
    command.env("AVOCADO_STONE_DATA_DIR", input_dir);

    if verbose {
        log_debug(&format!(
            "Running provision script: {}",
            provision_path.display()
        ));
    }

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to execute provision script '{provision_file}': {e}"))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Stream stdout in real-time
    let stdout_handle = std::thread::spawn(move || {
        for line in stdout_reader.lines().map_while(Result::ok) {
            println!("{line}");
        }
    });

    // Stream stderr in real-time
    let stderr_handle = std::thread::spawn(move || {
        for line in stderr_reader.lines().map_while(Result::ok) {
            eprintln!("{line}");
        }
    });

    // Wait for the process to complete
    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait for provision script '{provision_file}': {e}"))?;

    // Wait for the output threads to complete
    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    if !status.success() {
        return Err(format!(
            "Provision script '{}' failed with exit code {}",
            provision_file,
            status.code().unwrap_or(-1)
        ));
    }

    log_success(&format!(
        "Provision script '{provision_file}' completed successfully."
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_read_os_version_double_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="2.5.1"
PRETTY_NAME="Test Linux 1.0.0""#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let (result, _, _, _) = read_os_release_info(temp_dir.path()).unwrap();
        assert_eq!(result, "2.5.1");
    }

    #[test]
    fn test_read_os_version_single_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME='Test Linux'
VERSION='1.0.0'
ID=testlinux
VERSION_ID='3.0.0-beta'
PRETTY_NAME='Test Linux 1.0.0'"#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let (result, _, _, _) = read_os_release_info(temp_dir.path()).unwrap();
        assert_eq!(result, "3.0.0-beta");
    }

    #[test]
    fn test_read_os_version_no_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME=Test Linux
VERSION=1.0.0
ID=testlinux
VERSION_ID=4.2.1
PRETTY_NAME=Test Linux 1.0.0"#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let (result, _, _, _) = read_os_release_info(temp_dir.path()).unwrap();
        assert_eq!(result, "4.2.1");
    }

    #[test]
    fn test_read_os_version_missing_file() {
        let temp_dir = TempDir::new().unwrap();

        let result = read_os_release_info(temp_dir.path());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("OS release file 'os-release' not found")
        );
    }

    #[test]
    fn test_read_os_version_missing_version_id() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
PRETTY_NAME="Test Linux 1.0.0""#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let result = read_os_release_info(temp_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("VERSION_ID field not found"));
    }

    #[test]
    fn test_read_os_release_with_codename() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="2.5.1"
VERSION_CODENAME=jammy
PRETTY_NAME="Test Linux 1.0.0""#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let (version, codename, description, author) =
            read_os_release_info(temp_dir.path()).unwrap();
        assert_eq!(version, "2.5.1");
        assert_eq!(codename, "jammy");
        assert_eq!(description, "Test Linux 1.0.0");
        assert_eq!(author, "");
    }

    #[test]
    fn test_read_os_release_with_quoted_codename() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="3.0.0"
VERSION_CODENAME="focal"
PRETTY_NAME="Test Linux 1.0.0""#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let (version, codename, description, author) =
            read_os_release_info(temp_dir.path()).unwrap();
        assert_eq!(version, "3.0.0");
        assert_eq!(codename, "focal");
        assert_eq!(description, "Test Linux 1.0.0");
        assert_eq!(author, "");
    }

    #[test]
    fn test_read_os_release_without_codename() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="4.0.0"
PRETTY_NAME="Test Linux 1.0.0""#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let (version, codename, description, author) =
            read_os_release_info(temp_dir.path()).unwrap();
        assert_eq!(version, "4.0.0");
        assert_eq!(codename, "");
        assert_eq!(description, "Test Linux 1.0.0");
        assert_eq!(author, "");
    }

    #[test]
    fn test_read_os_release_missing_pretty_name() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="5.0.0"
VERSION_CODENAME=focal"#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let (version, codename, description, author) =
            read_os_release_info(temp_dir.path()).unwrap();
        assert_eq!(version, "5.0.0");
        assert_eq!(codename, "focal");
        assert_eq!(description, "");
        assert_eq!(author, "");
    }

    #[test]
    fn test_read_os_release_pretty_name_with_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="6.0.0"
VERSION_CODENAME=lunar
PRETTY_NAME="Ubuntu 22.04.3 LTS""#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let (version, codename, description, author) =
            read_os_release_info(temp_dir.path()).unwrap();
        assert_eq!(version, "6.0.0");
        assert_eq!(codename, "lunar");
        assert_eq!(description, "Ubuntu 22.04.3 LTS");
        assert_eq!(author, "");
    }

    #[test]
    fn test_read_os_release_pretty_name_single_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME='Test Linux'
VERSION='1.0.0'
ID=testlinux
VERSION_ID='7.0.0'
VERSION_CODENAME='mantic'
PRETTY_NAME='Debian GNU/Linux 12 (bookworm)'"#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let (version, codename, description, author) =
            read_os_release_info(temp_dir.path()).unwrap();
        assert_eq!(version, "7.0.0");
        assert_eq!(codename, "mantic");
        assert_eq!(description, "Debian GNU/Linux 12 (bookworm)");
        assert_eq!(author, "");
    }

    #[test]
    fn test_read_os_release_with_vendor_name() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="8.0.0"
VERSION_CODENAME=noble
PRETTY_NAME="Test Linux 8.0.0"
VENDOR_NAME="Acme Corporation""#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let (version, codename, description, author) =
            read_os_release_info(temp_dir.path()).unwrap();
        assert_eq!(version, "8.0.0");
        assert_eq!(codename, "noble");
        assert_eq!(description, "Test Linux 8.0.0");
        assert_eq!(author, "Acme Corporation");
    }

    #[test]
    fn test_read_os_release_vendor_name_single_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME='Test Linux'
VERSION='1.0.0'
ID=testlinux
VERSION_ID='9.0.0'
VERSION_CODENAME='oracular'
PRETTY_NAME='Test Linux 9.0.0'
VENDOR_NAME='Red Hat, Inc.'"#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let (version, codename, description, author) =
            read_os_release_info(temp_dir.path()).unwrap();
        assert_eq!(version, "9.0.0");
        assert_eq!(codename, "oracular");
        assert_eq!(description, "Test Linux 9.0.0");
        assert_eq!(author, "Red Hat, Inc.");
    }

    #[test]
    fn test_read_os_release_vendor_name_no_quotes() {
        let temp_dir = TempDir::new().unwrap();
        let os_release_content = r#"NAME=Test Linux
VERSION=1.0.0
ID=testlinux
VERSION_ID=10.0.0
VERSION_CODENAME=plucky
PRETTY_NAME=Test Linux 10.0.0
VENDOR_NAME=Canonical"#;

        fs::write(temp_dir.path().join("os-release"), os_release_content).unwrap();

        let (version, codename, description, author) =
            read_os_release_info(temp_dir.path()).unwrap();
        assert_eq!(version, "10.0.0");
        assert_eq!(codename, "plucky");
        assert_eq!(description, "Test Linux 10.0.0");
        assert_eq!(author, "Canonical");
    }
}
