use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub runtime: Runtime,
    pub storage_devices: HashMap<String, StorageDevice>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Runtime {
    pub platform: String,
    pub architecture: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StorageDevice {
    pub filename: String,
    pub build: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_conf: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub build_args: Vec<String>,
    pub devpath: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_size: Option<u32>,
    pub images: HashMap<String, Image>,
    pub partitions: Vec<Partition>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Image {
    String(String),
    Object {
        filename: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        build: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        build_args: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        files: Vec<FileEntry>,
    },
}

impl Image {
    pub fn filename(&self) -> &str {
        match self {
            Image::String(filename) => filename,
            Image::Object { filename, .. } => filename,
        }
    }

    pub fn build(&self) -> Option<&String> {
        match self {
            Image::String(_) => None,
            Image::Object { build, .. } => build.as_ref(),
        }
    }

    pub fn build_args(&self) -> &[String] {
        match self {
            Image::String(_) => &[],
            Image::Object { build_args, .. } => build_args,
        }
    }

    pub fn files(&self) -> &[FileEntry] {
        match self {
            Image::String(_) => &[],
            Image::Object { files, .. } => files,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FileEntry {
    String(String),
    Object {
        #[serde(rename = "in")]
        input: String,
        #[serde(rename = "out")]
        output: String,
    },
}

impl FileEntry {
    pub fn input_filename(&self) -> &str {
        match self {
            FileEntry::String(filename) => filename,
            FileEntry::Object { input, .. } => input,
        }
    }

    pub fn output_filename(&self) -> &str {
        match self {
            FileEntry::String(filename) => filename,
            FileEntry::Object { output, .. } => output,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Partition {
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<String>,
}

impl Manifest {
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read manifest file: {}", e))?;

        serde_json::from_str(&content).map_err(|e| format!("Failed to parse manifest JSON: {}", e))
    }
}
