use crate::fat;
use crate::log::*;
use crate::manifest::{BuildArgs, FatVariant, FileEntry, Image, Manifest};
use clap::Args;
use sha2::{Digest, Sha256};

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct BundleArgs {
    /// Path to the stone manifest JSON file
    #[arg(
        short = 'm',
        long = "manifest-path",
        value_name = "PATH",
        default_value = "manifest.json"
    )]
    pub manifest: PathBuf,

    /// Path to the OS release file to include
    #[arg(long = "os-release", value_name = "PATH")]
    pub os_release: PathBuf,

    /// Path to the initramfs OS release file (optional, for initramfs build ID)
    #[arg(long = "os-release-initrd", value_name = "PATH")]
    pub os_release_initrd: Option<PathBuf>,

    /// Path to the input directory (can be specified multiple times for search priority)
    #[arg(
        short = 'i',
        long = "input-dir",
        value_name = "DIR",
        default_value = "."
    )]
    pub input_dirs: Vec<PathBuf>,

    /// Path to the output .aos bundle file
    #[arg(
        short = 'o',
        long = "output",
        value_name = "PATH",
        default_value = "os-bundle.aos"
    )]
    pub output: PathBuf,

    /// Directory for intermediate build artifacts
    #[arg(long = "build-dir", value_name = "DIR")]
    pub build_dir: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

impl BundleArgs {
    pub fn execute(&self) -> Result<(), String> {
        bundle_command(
            &self.manifest,
            &self.os_release,
            self.os_release_initrd.as_deref(),
            &self.input_dirs,
            &self.output,
            self.build_dir.as_deref(),
            self.verbose,
        )
    }
}

/// Find a file in multiple input directories, searching in order
fn find_file_in_dirs(filename: &str, input_dirs: &[PathBuf]) -> Option<PathBuf> {
    for dir in input_dirs {
        let candidate = dir.join(filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Compute SHA256 hash of a file, returning the hex string
fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|e| format!("Failed to open '{}' for hashing: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn bundle_command(
    manifest_path: &Path,
    os_release_path: &Path,
    os_release_initrd_path: Option<&Path>,
    input_dirs: &[PathBuf],
    output_path: &Path,
    build_dir_override: Option<&Path>,
    verbose: bool,
) -> Result<(), String> {
    // Validate inputs exist
    if !manifest_path.exists() {
        return Err(format!(
            "Manifest file '{}' not found.",
            manifest_path.display()
        ));
    }
    if !os_release_path.exists() {
        return Err(format!(
            "OS release file '{}' not found.",
            os_release_path.display()
        ));
    }

    let manifest = Manifest::from_file(manifest_path)?;

    // Determine build directory
    let default_build_dir = output_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("_build");
    let build_dir = build_dir_override.unwrap_or(&default_build_dir);

    fs::create_dir_all(build_dir).map_err(|e| {
        format!(
            "Failed to create build directory '{}': {}",
            build_dir.display(),
            e
        )
    })?;

    let images_dir = build_dir.join("images");
    fs::create_dir_all(&images_dir).map_err(|e| {
        format!(
            "Failed to create images directory '{}': {}",
            images_dir.display(),
            e
        )
    })?;

    log_info(&format!(
        "Building OS bundle.\n  Manifest:   {}\n  Build dir:  {}\n  Output:     {}",
        manifest_path.display(),
        build_dir.display(),
        output_path.display()
    ));

    // Step 1: Copy all manifest inputs to build dir (like stone create)
    copy_manifest_inputs(
        &manifest,
        manifest_path,
        os_release_path,
        os_release_initrd_path,
        input_dirs,
        build_dir,
        verbose,
    )?;

    // Step 2: Build FAT images and collect built image artifacts
    let built_images = build_all_images(&manifest, input_dirs, build_dir, &images_dir, verbose)?;

    // Step 3: Collect all artifacts (built images + pre-existing images)
    let artifacts = collect_artifacts(&manifest, &built_images, input_dirs, &images_dir, verbose)?;

    // Step 4: Parse os-release for OS build ID
    let os_build_id = parse_os_release_field(os_release_path, "AVOCADO_OS_BUILD_ID")?;

    // Step 4b: Parse initramfs os-release for initramfs build ID (if provided)
    let initramfs_build_id = if let Some(initrd_path) = os_release_initrd_path {
        let id = parse_os_release_field(initrd_path, "AVOCADO_OS_BUILD_ID")?;
        if id.is_empty() { None } else { Some(id) }
    } else {
        None
    };

    // Step 5: Generate bundle.json
    let bundle_json = generate_bundle_json(&manifest, &artifacts, &os_build_id, initramfs_build_id.as_deref())?;
    let bundle_json_path = build_dir.join("bundle.json");
    let bundle_json_str = serde_json::to_string_pretty(&bundle_json)
        .map_err(|e| format!("Failed to serialize bundle.json: {e}"))?;
    fs::write(&bundle_json_path, &bundle_json_str)
        .map_err(|e| format!("Failed to write bundle.json: {e}"))?;

    if verbose {
        log_debug(&format!("Generated bundle.json:\n{bundle_json_str}"));
    }

    // Step 6: Package into .aos (tar.zst)
    package_aos(output_path, &bundle_json_path, &artifacts, verbose)?;

    log_success(&format!("OS bundle created: {}", output_path.display()));
    Ok(())
}

/// Represents a built/collected artifact ready for packaging
struct BundleArtifact {
    /// Name of the artifact (e.g., "boot", "rootfs")
    name: String,
    /// Path to the artifact file on disk
    path: PathBuf,
    /// Relative path inside the .aos archive (e.g., "images/boot.img")
    archive_path: String,
    /// SHA256 hash
    sha256: String,
}

/// Copy manifest inputs to the build directory (mirrors stone create behavior)
fn copy_manifest_inputs(
    manifest: &Manifest,
    manifest_path: &Path,
    os_release_path: &Path,
    os_release_initrd_path: Option<&Path>,
    input_dirs: &[PathBuf],
    build_dir: &Path,
    verbose: bool,
) -> Result<(), String> {
    // Copy the manifest itself
    let manifest_dest = build_dir.join("manifest.json");
    copy_file(manifest_path, &manifest_dest, verbose)?;

    // Copy os-release
    let os_release_dest = build_dir.join("os-release");
    copy_file(os_release_path, &os_release_dest, verbose)?;

    // Copy os-release-initrd (if provided)
    if let Some(initrd_path) = os_release_initrd_path {
        let initrd_dest = build_dir.join("os-release-initrd");
        copy_file(initrd_path, &initrd_dest, verbose)?;
    }

    // Copy fwup templates and provision scripts for provision compatibility
    for device in manifest.storage_devices.values() {
        if let Some(build_args) = &device.build_args
            && let Some(template) = build_args.fwup_template()
            && let Some(src) = find_file_in_dirs(template, input_dirs)
        {
            copy_file(&src, &build_dir.join(template), verbose)?;
        }

        // Copy image source files that are simple string references
        for image in device.images.values() {
            if let Image::String(filename) = image
                && let Some(src) = find_file_in_dirs(filename, input_dirs)
            {
                let dest = build_dir.join(filename);
                copy_path(&src, &dest, verbose)?;
            }
            // Copy fwup templates for images
            if let Some(ba) = image.build_args()
                && let Some(template) = ba.fwup_template()
                && let Some(src) = find_file_in_dirs(template, input_dirs)
            {
                copy_file(&src, &build_dir.join(template), verbose)?;
            }
            // Copy FAT source files (e.g., initramfs, bzImage) so provision can rebuild FAT images
            for file_entry in image.files() {
                let input_filename = file_entry.input_filename();
                if let Some(src) = find_file_in_dirs(input_filename, input_dirs) {
                    let dest = build_dir.join(input_filename);
                    copy_path(&src, &dest, verbose)?;
                }
            }
        }
    }

    // Copy provision file
    if let Some(provision_file) = &manifest.runtime.provision
        && let Some(src) = find_file_in_dirs(provision_file, input_dirs)
    {
        copy_file(&src, &build_dir.join(provision_file), verbose)?;
    }

    // Copy provision profile scripts
    if let Some(provision) = &manifest.provision {
        for profile in provision.profiles.values() {
            if let Some(src) = find_file_in_dirs(&profile.script, input_dirs) {
                copy_file(&src, &build_dir.join(&profile.script), verbose)?;
            }
        }
    }

    Ok(())
}

/// Build all images that have build_args (FAT images)
fn build_all_images(
    manifest: &Manifest,
    input_dirs: &[PathBuf],
    build_dir: &Path,
    images_dir: &Path,
    verbose: bool,
) -> Result<HashMap<String, PathBuf>, String> {
    let mut built = HashMap::new();

    for device in manifest.storage_devices.values() {
        for (image_name, image) in &device.images {
            match image {
                Image::Object {
                    out,
                    build_args: Some(BuildArgs::Fat { variant, files }),
                    size,
                    size_unit,
                    ..
                } => {
                    log_info(&format!("Building FAT image '{image_name}' -> '{out}'."));

                    let size_mb = convert_size_to_mb(*size, size_unit)?;
                    let fat_type = match variant {
                        FatVariant::Fat12 => fat::FatType::Fat12,
                        FatVariant::Fat16 => fat::FatType::Fat16,
                        FatVariant::Fat32 => fat::FatType::Fat32,
                    };

                    let fat_manifest = create_fat_manifest_with_resolved_paths(files, input_dirs)?;
                    let temp_manifest_path =
                        build_dir.join(format!("temp_manifest_{image_name}.json"));
                    let manifest_json = serde_json::to_string_pretty(&fat_manifest)
                        .map_err(|e| format!("Failed to serialize FAT manifest: {e}"))?;
                    fs::write(&temp_manifest_path, manifest_json)
                        .map_err(|e| format!("Failed to write temporary manifest: {e}"))?;

                    // Build into images/ dir for the bundle, and also into build_dir for provision
                    let output_in_images = images_dir.join(out);
                    let output_in_build = build_dir.join(out);
                    let base_path = PathBuf::from(".");

                    let options = fat::FatImageOptions::new()
                        .with_manifest_path(&temp_manifest_path)
                        .with_base_path(&base_path)
                        .with_output_path(&output_in_images)
                        .with_size_mebibytes(size_mb)
                        .with_fat_type(fat_type)
                        .with_verbose(verbose);

                    fat::create_fat_image(&options)?;
                    let _ = fs::remove_file(&temp_manifest_path);

                    // Also copy to build_dir so provision can find it at the same path as before
                    fs::copy(&output_in_images, &output_in_build)
                        .map_err(|e| format!("Failed to copy built image to build dir: {e}"))?;

                    log_success(&format!("Built FAT image '{out}'."));
                    built.insert(image_name.clone(), output_in_images);
                }
                _ => {
                    // Non-FAT images (string refs, fwup, or no build_args) are handled in collect_artifacts
                }
            }
        }
    }

    Ok(built)
}

/// Collect all artifacts that should go into the bundle.
/// Uses the update.os_artifacts section to determine which images to include.
fn collect_artifacts(
    manifest: &Manifest,
    built_images: &HashMap<String, PathBuf>,
    input_dirs: &[PathBuf],
    images_dir: &Path,
    verbose: bool,
) -> Result<Vec<BundleArtifact>, String> {
    let mut artifacts = Vec::new();

    let update = match &manifest.update {
        Some(u) => u,
        None => {
            // No update section - collect all images as artifacts
            log_warning("No 'update' section in manifest. Bundle will include all images.");
            return collect_all_images_as_artifacts(
                manifest,
                built_images,
                input_dirs,
                images_dir,
                verbose,
            );
        }
    };

    // Collect only the images referenced in os_artifacts
    for (artifact_name, artifact_ref) in &update.os_artifacts {
        let image_key = &artifact_ref.image_key;

        // Find this image in the manifest's storage_devices
        let image_path = if let Some(path) = built_images.get(image_key) {
            // Already built (FAT image)
            path.clone()
        } else {
            // Look for it as a pre-existing file
            let image = find_image_in_manifest(manifest, image_key)?;
            let filename = image.out();

            // Check if it's already in images_dir
            let in_images = images_dir.join(filename);
            if in_images.exists() {
                in_images
            } else {
                // Find in input dirs and copy to images/
                let src = find_file_in_dirs(filename, input_dirs).ok_or_else(|| {
                    format!(
                        "Image file '{}' for artifact '{}' not found in any input directory",
                        filename, artifact_name
                    )
                })?;
                let dest = images_dir.join(filename);
                copy_file(&src, &dest, verbose)?;
                dest
            }
        };

        let filename = image_path
            .file_name()
            .ok_or_else(|| format!("Invalid image path for artifact '{artifact_name}'"))?
            .to_string_lossy()
            .to_string();
        let archive_path = format!("images/{filename}");
        let sha256 = sha256_file(&image_path)?;

        if verbose {
            log_debug(&format!(
                "Artifact '{artifact_name}': {archive_path} (sha256: {sha256})"
            ));
        }

        artifacts.push(BundleArtifact {
            name: artifact_name.clone(),
            path: image_path,
            archive_path,
            sha256,
        });
    }

    Ok(artifacts)
}

/// Fallback: collect all images when no update section is present
fn collect_all_images_as_artifacts(
    manifest: &Manifest,
    built_images: &HashMap<String, PathBuf>,
    input_dirs: &[PathBuf],
    images_dir: &Path,
    verbose: bool,
) -> Result<Vec<BundleArtifact>, String> {
    let mut artifacts = Vec::new();

    for device in manifest.storage_devices.values() {
        for (image_name, image) in &device.images {
            let image_path = if let Some(path) = built_images.get(image_name) {
                path.clone()
            } else {
                let filename = image.out();
                let in_images = images_dir.join(filename);
                if in_images.exists() {
                    in_images
                } else if let Some(src) = find_file_in_dirs(filename, input_dirs) {
                    let dest = images_dir.join(filename);
                    copy_file(&src, &dest, verbose)?;
                    dest
                } else {
                    if verbose {
                        log_debug(&format!(
                            "Skipping image '{image_name}' - file '{}' not found",
                            filename
                        ));
                    }
                    continue;
                }
            };

            let filename = image_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let archive_path = format!("images/{filename}");
            let sha256 = sha256_file(&image_path)?;

            artifacts.push(BundleArtifact {
                name: image_name.clone(),
                path: image_path,
                archive_path,
                sha256,
            });
        }
    }

    Ok(artifacts)
}

/// Find an image by key across all storage devices in the manifest
fn find_image_in_manifest<'a>(
    manifest: &'a Manifest,
    image_key: &str,
) -> Result<&'a Image, String> {
    for device in manifest.storage_devices.values() {
        if let Some(image) = device.images.get(image_key) {
            return Ok(image);
        }
    }
    Err(format!(
        "Image key '{image_key}' not found in any storage device in the manifest"
    ))
}

/// Parse a field from an os-release file (KEY=VALUE format)
fn parse_os_release_field(path: &Path, field: &str) -> Result<String, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read os-release '{}': {}", path.display(), e))?;

    for line in content.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix(&format!("{field}=")) {
            // Strip surrounding quotes if present
            let value = value.trim_matches('"').trim_matches('\'');
            return Ok(value.to_string());
        }
    }

    // Not fatal - return empty string
    Ok(String::new())
}

/// Generate the bundle.json structure
fn generate_bundle_json(
    manifest: &Manifest,
    artifacts: &[BundleArtifact],
    os_build_id: &str,
    initramfs_build_id: Option<&str>,
) -> Result<serde_json::Value, String> {
    let update = manifest.update.as_ref();

    // Build the update.artifacts array for bundle.json
    let mut bundle_artifacts = Vec::new();
    for artifact in artifacts {
        let mut artifact_entry = serde_json::json!({
            "name": artifact.name,
            "file": artifact.archive_path,
            "sha256": artifact.sha256,
        });

        // Add slot_targets from the manifest's os_artifacts
        if let Some(update) = update
            && let Some(os_artifact) = update.os_artifacts.get(&artifact.name)
        {
            let slot_partitions = &os_artifact.slot_partitions;
            let mut slot_targets = serde_json::Map::new();

            // Determine slot identifiers based on update strategy
            let strategy = manifest
                .runtime
                .update_strategy
                .as_deref()
                .unwrap_or("uboot-ab");
            let slot_ids: Vec<&str> = match strategy {
                "tegra-ab" => vec!["0", "1"],
                _ => vec!["a", "b"],
            };

            for (idx, slot_id) in slot_ids.iter().enumerate() {
                if let Some(partition) = slot_partitions.get(idx) {
                    slot_targets.insert(
                        slot_id.to_string(),
                        serde_json::json!({ "partition": partition }),
                    );
                }
            }

            artifact_entry["slot_targets"] = serde_json::Value::Object(slot_targets);
        }

        bundle_artifacts.push(artifact_entry);
    }

    // Build the top-level bundle.json
    let mut bundle = serde_json::json!({
        "format_version": 1,
        "platform": manifest.runtime.platform,
        "architecture": manifest.runtime.architecture,
        "os_build_id": os_build_id,
    });

    if let Some(initramfs_id) = initramfs_build_id {
        bundle["initramfs_build_id"] = serde_json::json!(initramfs_id);
    }

    // Add update section if manifest has one
    if let Some(update) = update {
        let strategy = manifest
            .runtime
            .update_strategy
            .as_deref()
            .unwrap_or("uboot-ab");

        let mut update_section = serde_json::json!({
            "strategy": strategy,
            "slot_detection": serde_json::to_value(&update.slot_detection)
                .map_err(|e| format!("Failed to serialize slot_detection: {e}"))?,
            "artifacts": bundle_artifacts,
            "activate": serde_json::to_value(&update.activate)
                .map_err(|e| format!("Failed to serialize activate: {e}"))?,
        });

        if let Some(rollback) = &update.rollback {
            update_section["rollback"] = serde_json::to_value(rollback)
                .map_err(|e| format!("Failed to serialize rollback: {e}"))?;
        }

        bundle["update"] = update_section;
    }

    // Add layout section from storage_devices partitions
    for device in manifest.storage_devices.values() {
        if !device.partitions.is_empty() {
            let partitions: Vec<serde_json::Value> = device
                .partitions
                .iter()
                .map(|p| {
                    let mut part = serde_json::json!({});
                    if let Some(name) = &p.name {
                        part["name"] = serde_json::json!(name);
                    }
                    part["size"] = serde_json::json!(p.size);
                    part["size_unit"] = serde_json::json!(p.size_unit);
                    if let Some(offset) = p.offset {
                        part["offset"] = serde_json::json!(offset);
                    }
                    if let Some(offset_unit) = &p.offset_unit {
                        part["offset_unit"] = serde_json::json!(offset_unit);
                    }
                    if let Some(expand) = &p.expand {
                        part["expand"] = serde_json::json!(expand);
                    }
                    part
                })
                .collect();

            bundle["layout"] = serde_json::json!({
                "device": device.devpath,
                "partitions": partitions,
            });

            if let Some(block_size) = device.block_size {
                bundle["layout"]["block_size"] = serde_json::json!(block_size);
            }

            // Only include the first device's layout
            break;
        }
    }

    // Add verify section
    if !os_build_id.is_empty() {
        bundle["verify"] = serde_json::json!({
            "type": "os-release",
            "field": "AVOCADO_OS_BUILD_ID",
            "expected": os_build_id,
        });
    }

    // Add initramfs verify section
    if let Some(initramfs_id) = initramfs_build_id {
        bundle["verify_initramfs"] = serde_json::json!({
            "type": "os-release",
            "field": "AVOCADO_OS_BUILD_ID",
            "path": "/etc/os-release-initrd",
            "expected": initramfs_id,
        });
    }

    Ok(bundle)
}

/// Package everything into a .aos tar.zst archive
fn package_aos(
    output_path: &Path,
    bundle_json_path: &Path,
    artifacts: &[BundleArtifact],
    verbose: bool,
) -> Result<(), String> {
    // Create output directory if needed
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create output directory '{}': {}",
                parent.display(),
                e
            )
        })?;
    }

    let output_file = fs::File::create(output_path).map_err(|e| {
        format!(
            "Failed to create output file '{}': {}",
            output_path.display(),
            e
        )
    })?;

    let zst_encoder = zstd::Encoder::new(output_file, 3)
        .map_err(|e| format!("Failed to create zstd encoder: {e}"))?;

    let mut tar_builder = tar::Builder::new(zst_encoder);

    // Add bundle.json at the root
    if verbose {
        log_debug("Adding bundle.json to archive.");
    }
    tar_builder
        .append_path_with_name(bundle_json_path, "bundle.json")
        .map_err(|e| format!("Failed to add bundle.json to archive: {e}"))?;

    // Add each artifact
    for artifact in artifacts {
        if verbose {
            log_debug(&format!(
                "Adding {} -> {}",
                artifact.path.display(),
                artifact.archive_path
            ));
        }
        tar_builder
            .append_path_with_name(&artifact.path, &artifact.archive_path)
            .map_err(|e| {
                format!(
                    "Failed to add '{}' to archive: {}",
                    artifact.archive_path, e
                )
            })?;
    }

    // Finish the tar, then finish zstd
    let zst_encoder = tar_builder
        .into_inner()
        .map_err(|e| format!("Failed to finalize tar archive: {e}"))?;
    zst_encoder
        .finish()
        .map_err(|e| format!("Failed to finalize zstd compression: {e}"))?;

    Ok(())
}

/// Convert size value to mebibytes based on unit string
fn convert_size_to_mb(size: i64, size_unit: &str) -> Result<u64, String> {
    let size_mb = match size_unit.to_lowercase().as_str() {
        "bytes" | "byte" | "b" => size as f64 / (1024.0 * 1024.0),
        "kilobytes" | "kilobyte" | "kb" => size as f64 / 1024.0,
        "kibibytes" | "kibibyte" | "kib" => size as f64 / 1024.0,
        "megabytes" | "megabyte" | "mb" => size as f64,
        "mebibytes" | "mebibyte" | "mib" => size as f64,
        "gigabytes" | "gigabyte" | "gb" => size as f64 * 1024.0,
        "gibibytes" | "gibibyte" | "gib" => size as f64 * 1024.0,
        _ => return Err(format!("Unsupported size unit: {size_unit}")),
    };

    if size_mb <= 0.0 {
        return Err("Image size must be positive".to_string());
    }

    Ok(size_mb.ceil() as u64)
}

/// Resolve file paths for FAT manifest entries
fn create_fat_manifest_with_resolved_paths(
    files: &[FileEntry],
    input_dirs: &[PathBuf],
) -> Result<fat::Manifest, String> {
    let mut fat_files = Vec::new();

    for entry in files {
        let (input_filename, output_name) = match entry {
            FileEntry::String(filename) => (filename.as_str(), filename.clone()),
            FileEntry::Object { input, output } => (input.as_str(), output.clone()),
        };

        let resolved_path = find_file_in_dirs(input_filename, input_dirs).ok_or_else(|| {
            format!("File '{input_filename}' not found in any input directory for FAT image")
        })?;

        fat_files.push(fat::FileEntry {
            filename: Some(resolved_path.to_string_lossy().to_string()),
            output: Some(output_name),
        });
    }

    Ok(fat::Manifest {
        files: fat_files,
        directories: None,
    })
}

fn copy_path(input_path: &Path, output_path: &Path, verbose: bool) -> Result<(), String> {
    if !input_path.exists() {
        return Err(format!("Input path '{}' not found.", input_path.display()));
    }

    if input_path.is_dir() {
        copy_directory(input_path, output_path, verbose)
    } else {
        copy_file(input_path, output_path, verbose)
    }
}

fn copy_directory(input_dir: &Path, output_dir: &Path, verbose: bool) -> Result<(), String> {
    fs::create_dir_all(output_dir).map_err(|e| {
        format!(
            "Failed to create directory '{}': {}",
            output_dir.display(),
            e
        )
    })?;

    let entries = fs::read_dir(input_dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", input_dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let input_child = entry.path();
        let output_child = output_dir.join(entry.file_name());

        if input_child.is_dir() {
            copy_directory(&input_child, &output_child, verbose)?;
        } else {
            copy_file(&input_child, &output_child, verbose)?;
        }
    }

    Ok(())
}

fn copy_file(input_path: &Path, output_path: &Path, verbose: bool) -> Result<(), String> {
    if !input_path.exists() {
        return Err(format!("Input file '{}' not found.", input_path.display()));
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory '{}': {}", parent.display(), e))?;
    }

    fs::copy(input_path, output_path).map_err(|e| {
        format!(
            "Failed to copy '{}' to '{}': {}",
            input_path.display(),
            output_path.display(),
            e
        )
    })?;

    if verbose {
        log_debug(&format!(
            "Copied:\n  {}\n  {}",
            input_path.display(),
            output_path.display()
        ));
    }

    Ok(())
}
