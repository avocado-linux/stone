//! FAT filesystem image creation from JSON manifests.
//!
//! This module provides functionality to create FAT filesystem images based on
//! JSON manifest files that describe the files and directories to include.
//!
//! # Example
//!
//! ```rust
//! use stone::fat::{FatImageOptions, FatType, create_fat_image};
//! use std::path::PathBuf;
//!
//! let options = FatImageOptions::new()
//!     .with_manifest_path("files.json")
//!     .with_base_path("./source")
//!     .with_output_path("filesystem.img")
//!     .with_size_mb(32)
//!     .with_label("MYFS")
//!     .with_fat_type(FatType::Fat32)
//!     .with_verbose(true);
//!
//! create_fat_image(&options)?;
//! ```
//!
//! # Manifest Format
//!
//! The JSON manifest should have the following structure:
//!
//! ```json
//! {
//!   "directories": ["boot", "config"],
//!   "files": [
//!     {
//!       "filename": "kernel.bin",
//!       "output": "boot/kernel.bin"
//!     },
//!     {
//!       "filename": "config.txt"
//!     }
//!   ]
//! }
//! ```

use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

// Custom trait that combines Read, Write, and Seek
trait ReadWriteSeek: Read + Write + Seek {}
impl<T: Read + Write + Seek> ReadWriteSeek for T {}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FatType {
    Fat12,
    Fat16,
    Fat32,
}

impl FatType {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "fat12" => Ok(FatType::Fat12),
            "fat16" => Ok(FatType::Fat16),
            "fat32" => Ok(FatType::Fat32),
            _ => Err(format!("Invalid FAT type: {}", s)),
        }
    }
}

impl Default for FatType {
    fn default() -> Self {
        FatType::Fat32
    }
}

#[derive(Debug, Deserialize)]
struct FileEntry {
    filename: Option<String>,
    output: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    files: Vec<FileEntry>,
    directories: Option<Vec<String>>,
}

pub struct FatImageOptions {
    pub manifest_path: PathBuf,
    pub base_path: PathBuf,
    pub output_path: PathBuf,
    pub size_mb: u64,
    pub label: String,
    pub fat_type: FatType,
    pub verbose: bool,
}

impl Default for FatImageOptions {
    fn default() -> Self {
        Self {
            manifest_path: PathBuf::from("manifest.json"),
            base_path: PathBuf::from("."),
            output_path: PathBuf::from("output.img"),
            size_mb: 16,
            label: "FATFS".to_string(),
            fat_type: FatType::default(),
            verbose: false,
        }
    }
}

impl FatImageOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_manifest_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.manifest_path = path.into();
        self
    }

    pub fn with_base_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.base_path = path.into();
        self
    }

    pub fn with_output_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.output_path = path.into();
        self
    }

    pub fn with_size_mb(mut self, size_mb: u64) -> Self {
        self.size_mb = size_mb;
        self
    }

    pub fn with_label<S: Into<String>>(mut self, label: S) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_fat_type(mut self, fat_type: FatType) -> Self {
        self.fat_type = fat_type;
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

pub fn create_fat_image(options: &FatImageOptions) -> Result<(), String> {
    let mut base_path = options.base_path.clone();
    if base_path.is_relative() {
        base_path = std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?
            .join(&base_path);
    }

    if options.verbose {
        println!("Reading manifest: {}", options.manifest_path.display());
    }

    let json_str = fs::read_to_string(&options.manifest_path).map_err(|e| {
        format!(
            "Failed to read manifest file '{}': {}",
            options.manifest_path.display(),
            e
        )
    })?;

    let manifest: Manifest = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse manifest file: {}", e))?;

    if options.verbose {
        println!("Generating FAT image: {}", options.output_path.display());
    }

    generate_fat_image(options, &manifest, &base_path)?;

    if options.verbose {
        println!("FAT image generation complete.");
    }

    Ok(())
}

fn generate_fat_image(
    options: &FatImageOptions,
    manifest: &Manifest,
    base: &Path,
) -> Result<(), String> {
    // Create and preallocate output file
    let img_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&options.output_path)
        .map_err(|e| {
            format!(
                "Failed to open output file '{}': {}",
                options.output_path.display(),
                e
            )
        })?;

    img_file
        .set_len(options.size_mb * 1024 * 1024)
        .map_err(|e| format!("Failed to set image size: {}", e))?;

    // Keep the file in a box to satisfy the 'static lifetime requirement
    let mut boxed_file: Box<dyn ReadWriteSeek> = Box::new(img_file);

    let fat_type = match options.fat_type {
        FatType::Fat12 => fatfs::FatType::Fat12,
        FatType::Fat16 => fatfs::FatType::Fat16,
        FatType::Fat32 => fatfs::FatType::Fat32,
    };

    // Format the volume
    let mut label_bytes = [b' '; 11];
    let label_len = options.label.len().min(11);
    label_bytes[..label_len].copy_from_slice(&options.label.as_bytes()[..label_len]);

    let format_options = fatfs::FormatVolumeOptions::new()
        .volume_label(label_bytes)
        .fat_type(fat_type);

    fatfs::format_volume(&mut boxed_file, format_options)
        .map_err(|e| format!("Failed to format volume: {}", e))?;

    // Rewind the file for filesystem operations
    boxed_file
        .seek(SeekFrom::Start(0))
        .map_err(|e| format!("Failed to seek in image file: {}", e))?;

    // Create filesystem
    let fs = fatfs::FileSystem::new(boxed_file, fatfs::FsOptions::new())
        .map_err(|e| format!("Failed to create filesystem: {}", e))?;
    let root_dir = fs.root_dir();

    // Create directories first
    if let Some(directories) = &manifest.directories {
        for dir_path in directories {
            if options.verbose {
                println!("Creating directory: {}", dir_path);
            }
            create_directory_path(&root_dir, dir_path)?;
        }
    }

    // Add files
    for entry in manifest.files.iter() {
        let input_path = entry
            .filename
            .as_ref()
            .unwrap_or_else(|| entry.output.as_ref().unwrap());
        let output_path = entry
            .output
            .as_ref()
            .unwrap_or_else(|| entry.filename.as_ref().unwrap());

        if options.verbose {
            println!("Adding file: {} -> {}", input_path, output_path);
        }

        add_file_to_fat(&root_dir, base, input_path, output_path)?;
    }

    Ok(())
}

fn create_directory_path(
    root_dir: &fatfs::Dir<Box<dyn ReadWriteSeek>>,
    dir_path: &str,
) -> Result<(), String> {
    let components_vec: Vec<_> = Path::new(dir_path).components().collect();
    let mut dir = root_dir.clone();

    for comp in &components_vec {
        if let Component::RootDir = comp {
            continue;
        }
        let name = comp.as_os_str().to_str().ok_or("Invalid UTF-8 in path")?;
        dir = dir
            .create_dir(name)
            .or_else(|_| dir.open_dir(name))
            .map_err(|e| format!("Failed to create directory '{}': {}", name, e))?;
    }

    Ok(())
}

fn add_file_to_fat(
    root_dir: &fatfs::Dir<Box<dyn ReadWriteSeek>>,
    base: &Path,
    input_path: &str,
    output_path: &str,
) -> Result<(), String> {
    let full_input_path = base.join(input_path);
    let file_data = fs::read(&full_input_path).map_err(|e| {
        format!(
            "Failed to read input file '{}': {}",
            full_input_path.display(),
            e
        )
    })?;

    let components_vec: Vec<_> = Path::new(output_path).components().collect();
    let mut dir = root_dir.clone();

    // Navigate to the parent directory, creating directories as needed
    for comp in &components_vec[..components_vec.len().saturating_sub(1)] {
        if let Component::RootDir = comp {
            continue;
        }
        let name = comp.as_os_str().to_str().ok_or("Invalid UTF-8 in path")?;
        dir = dir
            .create_dir(name)
            .or_else(|_| dir.open_dir(name))
            .map_err(|e| format!("Failed to create directory '{}': {}", name, e))?;
    }

    // Create and write the file
    let file_name = Path::new(output_path)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Invalid file name")?;

    let mut fat_file = dir
        .create_file(file_name)
        .map_err(|e| format!("Failed to create file '{}': {}", file_name, e))?;

    fat_file
        .write_all(&file_data)
        .map_err(|e| format!("Failed to write to file '{}': {}", file_name, e))?;

    Ok(())
}
