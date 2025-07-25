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
    #[serde(default)]
    pub build_args: Vec<String>,
    pub devpath: String,
    pub images: HashMap<String, Image>,
    pub partitions: Vec<Partition>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Image {
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    #[serde(default)]
    pub build_args: Vec<String>,
    #[serde(default)]
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileEntry {
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Partition {
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocks: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_mb: Option<String>,
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
