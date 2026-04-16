use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub enum FatVariant {
    #[serde(rename = "FAT12")]
    Fat12,
    #[serde(rename = "FAT16")]
    Fat16,
    #[serde(rename = "FAT32")]
    Fat32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum BuildArgs {
    #[serde(rename = "fat")]
    Fat {
        variant: FatVariant,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        files: Vec<FileEntry>,
    },
    #[serde(rename = "fwup")]
    Fwup {
        template: String, // Path to template file
    },
}

impl BuildArgs {
    pub fn build_type(&self) -> &str {
        match self {
            BuildArgs::Fat { .. } => "fat",
            BuildArgs::Fwup { .. } => "fwup",
        }
    }

    pub fn fwup_template(&self) -> Option<&str> {
        match self {
            BuildArgs::Fwup { template } => Some(template),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn fat_files(&self) -> &[FileEntry] {
        match self {
            BuildArgs::Fat { files, .. } => files,
            _ => &[],
        }
    }

    pub fn fat_variant(&self) -> Option<&FatVariant> {
        match self {
            BuildArgs::Fat { variant, .. } => Some(variant),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub runtime: Runtime,
    pub storage_devices: std::collections::HashMap<String, StorageDevice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provision: Option<Provision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update: Option<Update>,
}

// --- Update section: declares how OS artifacts map to A/B slots for OTA ---

#[derive(Debug, Deserialize, Serialize)]
pub struct Update {
    pub slot_detection: SlotDetection,
    pub os_artifacts: HashMap<String, OsArtifactRef>,
    pub activate: SlotActions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback: Option<SlotActions>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum SlotDetection {
    #[serde(rename = "uboot-env")]
    UbootEnv { var: String },
    #[serde(rename = "command")]
    Command { command: Vec<String> },
    #[serde(rename = "sdboot-efi")]
    SdbootEfi {
        /// Map from GPT partition UUID -> slot name (e.g. {"<uuid>": "a", "<uuid>": "b"})
        partitions: HashMap<String, String>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OsArtifactRef {
    pub image_key: String,
    pub slot_partitions: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum SlotAction {
    #[serde(rename = "uboot-env")]
    UbootEnv { set: HashMap<String, String> },
    #[serde(rename = "command")]
    Command { command: Vec<String> },
    #[serde(rename = "mbr-switch")]
    MbrSwitch {
        devpath: String,
        slot_layouts: HashMap<String, Vec<String>>,
    },
    #[serde(rename = "efibootmgr")]
    Efibootmgr {
        /// Map from slot name -> EFI boot entry label (e.g. {"a": "boot-a", "b": "boot-b"})
        slot_entries: HashMap<String, String>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum SlotActions {
    Single(SlotAction),
    Multiple(Vec<SlotAction>),
}

impl SlotActions {
    pub fn as_vec(&self) -> Vec<&SlotAction> {
        match self {
            SlotActions::Single(a) => vec![a],
            SlotActions::Multiple(v) => v.iter().collect(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Runtime {
    pub platform: String,
    pub architecture: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provision_default: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_strategy: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Provision {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envs: Option<HashMap<String, HashMap<String, String>>>,
    /// Extra files to include alongside provision profile scripts (e.g., shared
    /// helper libraries that scripts source at runtime).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    pub profiles: HashMap<String, ProvisionProfile>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProvisionProfile {
    pub script: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envs: Option<Vec<ProvisionEnv>>,
    /// Declares capabilities this profile requires from the host environment.
    /// Known values: "usb" (USB device passthrough for direct device flashing).
    /// Scripts should check AVOCADO_USB_PASSTHROUGH env var to adapt at runtime.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ProvisionEnv {
    Named(String),
    Inline(HashMap<String, String>),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StorageDevice {
    pub out: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_args: Option<BuildArgs>,
    pub devpath: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    pub images: std::collections::HashMap<String, Image>,
    pub partitions: Vec<Partition>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Image {
    String(String),
    Object {
        out: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        build_args: Option<BuildArgs>,
        size: i64,
        size_unit: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        block_size: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        uuid: Option<String>,
    },
}

impl Image {
    pub fn out(&self) -> &str {
        match self {
            Image::String(filename) => filename,
            Image::Object { out, .. } => out,
        }
    }

    pub fn build(&self) -> Option<String> {
        match self {
            Image::String(_) => None,
            Image::Object { build_args, .. } => build_args
                .as_ref()
                .map(|args| args.build_type().to_string()),
        }
    }

    pub fn build_args(&self) -> Option<&BuildArgs> {
        match self {
            Image::String(_) => None,
            Image::Object { build_args, .. } => build_args.as_ref(),
        }
    }

    pub fn files(&self) -> &[FileEntry] {
        match self {
            Image::String(_) => &[],
            Image::Object { build_args, .. } => build_args
                .as_ref()
                .map(|args| args.fat_files())
                .unwrap_or(&[]),
        }
    }

    pub fn size(&self) -> Option<i64> {
        match self {
            Image::String(_) => None,
            Image::Object { size, .. } => Some(*size),
        }
    }

    pub fn size_unit(&self) -> Option<&str> {
        match self {
            Image::String(_) => None,
            Image::Object { size_unit, .. } => Some(size_unit),
        }
    }

    pub fn block_size(&self) -> Option<u32> {
        match self {
            Image::String(_) => None,
            Image::Object { block_size, .. } => *block_size,
        }
    }

    pub fn uuid(&self) -> Option<&str> {
        match self {
            Image::String(_) => None,
            Image::Object { uuid, .. } => uuid.as_deref(),
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Partition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_redundant: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_redundant_unit: Option<String>,
    pub size: i64,
    pub size_unit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<String>,
}

impl Manifest {
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            format!(
                "[ERROR] Failed to read manifest file '{}': {}",
                path.display(),
                e
            )
        })?;

        serde_json::from_str(&content).map_err(|e| {
            format!(
                "[ERROR] Failed to parse manifest JSON '{}': {}",
                path.display(),
                e
            )
        })
    }

    /// Load a manifest from a base file, then deep-merge each overlay file
    /// on top in order (left-to-right, last wins). The merged JSON is then
    /// deserialized into a typed `Manifest`.
    pub fn from_file_with_overlays(
        base_path: &std::path::Path,
        overlay_paths: &[std::path::PathBuf],
    ) -> Result<Self, String> {
        if overlay_paths.is_empty() {
            return Self::from_file(base_path);
        }

        let (manifest, _) = Self::load_and_merge(base_path, overlay_paths)?;
        Ok(manifest)
    }

    /// Load a manifest with overlays, returning both the typed Manifest and
    /// the pretty-printed merged JSON string (for commands that need to write
    /// the merged manifest to disk).
    pub fn load_and_merge(
        base_path: &std::path::Path,
        overlay_paths: &[std::path::PathBuf],
    ) -> Result<(Self, Option<String>), String> {
        if overlay_paths.is_empty() {
            let manifest = Self::from_file(base_path)?;
            return Ok((manifest, None));
        }

        let base_content = std::fs::read_to_string(base_path).map_err(|e| {
            format!(
                "[ERROR] Failed to read manifest file '{}': {}",
                base_path.display(),
                e
            )
        })?;
        let mut merged: Value = serde_json::from_str(&base_content).map_err(|e| {
            format!(
                "[ERROR] Failed to parse manifest JSON '{}': {}",
                base_path.display(),
                e
            )
        })?;

        for overlay_path in overlay_paths {
            let overlay_content = std::fs::read_to_string(overlay_path).map_err(|e| {
                format!(
                    "[ERROR] Failed to read overlay file '{}': {}",
                    overlay_path.display(),
                    e
                )
            })?;
            let overlay_value: Value = serde_json::from_str(&overlay_content).map_err(|e| {
                format!(
                    "[ERROR] Failed to parse overlay JSON '{}': {}",
                    overlay_path.display(),
                    e
                )
            })?;
            deep_merge_json(&mut merged, overlay_value);
        }

        let merged_json =
            serde_json::to_string_pretty(&merged).map_err(|e| {
                format!("[ERROR] Failed to serialize merged manifest: {}", e)
            })?;

        let manifest: Self = serde_json::from_value(merged).map_err(|e| {
            format!(
                "[ERROR] Merged manifest (base '{}' + {} overlay(s)) is invalid: {}",
                base_path.display(),
                overlay_paths.len(),
                e
            )
        })?;

        Ok((manifest, Some(merged_json)))
    }

    pub fn get_provision_profile(&self, profile_name: &str) -> Option<&ProvisionProfile> {
        self.provision.as_ref()?.profiles.get(profile_name)
    }

    pub fn get_provision_default(&self) -> Option<&str> {
        self.runtime.provision_default.as_deref()
    }
}

impl Provision {
    pub fn resolve_envs(
        &self,
        profile: &ProvisionProfile,
    ) -> Result<HashMap<String, String>, String> {
        let mut resolved_envs = HashMap::new();

        if let Some(envs) = &profile.envs {
            for env in envs {
                match env {
                    ProvisionEnv::Named(env_name) => {
                        if let Some(named_envs) = &self.envs {
                            if let Some(env_block) = named_envs.get(env_name) {
                                for (key, value) in env_block {
                                    resolved_envs.insert(key.clone(), value.clone());
                                }
                            } else {
                                return Err(format!(
                                    "[ERROR] Named environment block '{env_name}' not found in provision.envs."
                                ));
                            }
                        } else {
                            return Err(format!(
                                "[ERROR] Named environment block '{env_name}' referenced but no provision.envs defined."
                            ));
                        }
                    }
                    ProvisionEnv::Inline(inline_envs) => {
                        for (key, value) in inline_envs {
                            resolved_envs.insert(key.clone(), value.clone());
                        }
                    }
                }
            }
        }

        Ok(resolved_envs)
    }

    pub fn expand_env_vars(&self, envs: &HashMap<String, String>) -> HashMap<String, String> {
        use crate::log::log_warning;

        let mut expanded = HashMap::new();
        let mut missing_vars = Vec::new();

        for (key, value) in envs {
            let (expanded_value, missing) = self.expand_single_env_var(value);
            missing_vars.extend(missing);
            expanded.insert(key.clone(), expanded_value);
        }

        if !missing_vars.is_empty() {
            // Deduplicate missing vars (same var might be referenced multiple times)
            missing_vars.sort();
            missing_vars.dedup();
            log_warning(&format!(
                "The following environment variables are referenced but not set or empty in the caller's environment: {}",
                missing_vars.join(", ")
            ));
        }

        expanded
    }

    fn expand_single_env_var(&self, value: &str) -> (String, Vec<String>) {
        let mut result = value.to_string();
        let mut missing_vars = Vec::new();
        let mut search_start = 0;

        // Simple regex-like replacement for ${VAR_NAME} patterns
        while let Some(relative_start) = result[search_start..].find("${") {
            let start = search_start + relative_start;
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 2..start + end];
                match std::env::var(var_name) {
                    Ok(replacement) if !replacement.is_empty() => {
                        result.replace_range(start..start + end + 1, &replacement);
                        // Continue searching from where we left off (replacement might be shorter/longer)
                        search_start = start + replacement.len();
                    }
                    _ => {
                        // Variable not set or empty - replace with empty string and record warning
                        missing_vars.push(var_name.to_string());
                        result.replace_range(start..start + end + 1, "");
                        // search_start stays at `start` since we removed the ${VAR}
                    }
                }
            } else {
                break;
            }
        }

        (result, missing_vars)
    }
}

/// Deep-merge two JSON values. Objects are merged recursively (overlay keys
/// win on conflict). Arrays of named objects (elements with a `"name"` string
/// field) are merged by matching names. All other values (scalars, unnamed
/// arrays) in `overlay` replace `base` entirely.
pub fn deep_merge_json(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_val) in overlay_map {
                let entry = base_map.entry(key).or_insert(Value::Null);
                deep_merge_json(entry, overlay_val);
            }
        }
        (Value::Array(base_arr), Value::Array(overlay_arr)) => {
            // Name-based merge: if all overlay elements are objects with "name" fields,
            // match to base elements by name and deep-merge individually
            let all_named = !overlay_arr.is_empty()
                && overlay_arr
                    .iter()
                    .all(|v| v.get("name").and_then(|n| n.as_str()).is_some());
            if all_named {
                for overlay_elem in overlay_arr {
                    let name = overlay_elem.get("name").unwrap().as_str().unwrap();
                    if let Some(base_elem) = base_arr
                        .iter_mut()
                        .find(|b| b.get("name").and_then(|n| n.as_str()) == Some(name))
                    {
                        deep_merge_json(base_elem, overlay_elem);
                    } else {
                        base_arr.push(overlay_elem);
                    }
                }
            } else {
                *base_arr = overlay_arr;
            }
        }
        (base, overlay) => {
            *base = overlay;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_args_serialization() {
        let fat_args = BuildArgs::Fat {
            variant: FatVariant::Fat32,
            files: vec![],
        };

        let serialized = serde_json::to_value(&fat_args).unwrap();
        assert_eq!(serialized["type"], "fat");
        assert_eq!(serialized["variant"], "FAT32");
    }

    #[test]
    fn test_build_args_deserialization() {
        let json_str = r#"{"type":"fwup","template":"my_template.conf"}"#;
        let deserialized: BuildArgs = serde_json::from_str(json_str).unwrap();

        match deserialized {
            BuildArgs::Fwup { template } => {
                assert_eq!(template, "my_template.conf");
            }
            _ => panic!("Expected Fwup variant"),
        }
    }

    #[test]
    fn test_build_args_type_access() {
        let fat_args = BuildArgs::Fat {
            variant: FatVariant::Fat16,
            files: vec![],
        };
        assert_eq!(fat_args.build_type(), "fat");

        let fwup_args = BuildArgs::Fwup {
            template: "config.conf".to_string(),
        };
        assert_eq!(fwup_args.build_type(), "fwup");
    }

    #[test]
    fn test_image_build_method() {
        let image = Image::Object {
            out: "test.img".to_string(),
            build_args: Some(BuildArgs::Fat {
                variant: FatVariant::Fat32,
                files: vec![],
            }),
            size: 100,
            size_unit: "megabytes".to_string(),
            block_size: None,
            uuid: None,
        };

        assert_eq!(image.build().unwrap(), "fat");

        let string_image = Image::String("simple.img".to_string());
        assert!(string_image.build().is_none());
    }

    #[test]
    fn test_image_block_size_and_uuid() {
        // Test Image::Object with block_size and uuid
        let image_with_disk_info = Image::Object {
            out: "disk.img".to_string(),
            build_args: Some(BuildArgs::Fwup {
                template: "disk.conf".to_string(),
            }),
            size: 512,
            size_unit: "megabytes".to_string(),
            block_size: Some(4096),
            uuid: Some("12345678-1234-1234-1234-123456789abc".to_string()),
        };

        assert_eq!(image_with_disk_info.block_size(), Some(4096));
        assert_eq!(
            image_with_disk_info.uuid(),
            Some("12345678-1234-1234-1234-123456789abc")
        );

        // Test Image::Object without block_size and uuid
        let image_without_disk_info = Image::Object {
            out: "simple.img".to_string(),
            build_args: None,
            size: 256,
            size_unit: "megabytes".to_string(),
            block_size: None,
            uuid: None,
        };

        assert_eq!(image_without_disk_info.block_size(), None);
        assert_eq!(image_without_disk_info.uuid(), None);

        // Test Image::String
        let string_image = Image::String("file.img".to_string());
        assert_eq!(string_image.block_size(), None);
        assert_eq!(string_image.uuid(), None);
    }

    #[test]
    fn test_storage_device_with_build_args() {
        let json_str = r#"{
            "out": "disk.img",
            "devpath": "/dev/sda",
            "build_args": {
                "type": "fwup",
                "template": "config.conf"
            },
            "images": {},
            "partitions": []
        }"#;

        let device: StorageDevice = serde_json::from_str(json_str).unwrap();

        assert_eq!(device.out, "disk.img");
        assert_eq!(device.devpath, "/dev/sda");

        let build_args = device.build_args.unwrap();
        assert_eq!(build_args.build_type(), "fwup");
        assert_eq!(build_args.fwup_template().unwrap(), "config.conf");
    }

    #[test]
    fn test_fat_build_args_with_files() {
        let fat_args = BuildArgs::Fat {
            variant: FatVariant::Fat32,
            files: vec![
                FileEntry::String("file1.txt".to_string()),
                FileEntry::Object {
                    input: "source.bin".to_string(),
                    output: "dest.bin".to_string(),
                },
            ],
        };

        assert_eq!(fat_args.build_type(), "fat");
        assert_eq!(fat_args.fat_files().len(), 2);
        assert_eq!(fat_args.fat_files()[0].input_filename(), "file1.txt");
        assert_eq!(fat_args.fat_files()[1].input_filename(), "source.bin");
    }

    #[test]
    fn test_runtime_with_provision() {
        let runtime = Runtime {
            platform: "linux".to_string(),
            architecture: "x86_64".to_string(),
            provision: Some("provision.sh".to_string()),
            provision_default: None,
            update_strategy: None,
        };

        let serialized = serde_json::to_value(&runtime).unwrap();
        assert_eq!(serialized["platform"], "linux");
        assert_eq!(serialized["architecture"], "x86_64");
        assert_eq!(serialized["provision"], "provision.sh");

        let json_str = r#"{"platform":"linux","architecture":"x86_64","provision":"provision.sh"}"#;
        let deserialized: Runtime = serde_json::from_str(json_str).unwrap();
        assert_eq!(deserialized.platform, "linux");
        assert_eq!(deserialized.architecture, "x86_64");
        assert_eq!(deserialized.provision, Some("provision.sh".to_string()));
    }

    #[test]
    fn test_runtime_without_provision() {
        let runtime = Runtime {
            platform: "linux".to_string(),
            architecture: "x86_64".to_string(),
            provision: None,
            provision_default: None,
            update_strategy: None,
        };

        let serialized = serde_json::to_value(&runtime).unwrap();
        assert_eq!(serialized["platform"], "linux");
        assert_eq!(serialized["architecture"], "x86_64");
        assert!(!serialized.as_object().unwrap().contains_key("provision"));

        let json_str = r#"{"platform":"linux","architecture":"x86_64"}"#;
        let deserialized: Runtime = serde_json::from_str(json_str).unwrap();
        assert_eq!(deserialized.platform, "linux");
        assert_eq!(deserialized.architecture, "x86_64");
        assert_eq!(deserialized.provision, None);
    }

    #[test]
    fn test_partition_with_name_and_redundant_offset() {
        let json_str = r#"{
            "name": "uboot-env",
            "image": "uboot_env",
            "offset": 1,
            "offset_unit": "mebibytes",
            "offset_redundant": 1152,
            "offset_redundant_unit": "kibibytes",
            "size": 128,
            "size_unit": "kibibytes"
        }"#;

        let partition: Partition = serde_json::from_str(json_str).unwrap();

        assert_eq!(partition.name, Some("uboot-env".to_string()));
        assert_eq!(partition.image, Some("uboot_env".to_string()));
        assert_eq!(partition.offset, Some(1));
        assert_eq!(partition.offset_unit, Some("mebibytes".to_string()));
        assert_eq!(partition.offset_redundant, Some(1152));
        assert_eq!(
            partition.offset_redundant_unit,
            Some("kibibytes".to_string())
        );
        assert_eq!(partition.size, 128);
        assert_eq!(partition.size_unit, "kibibytes");
    }

    #[test]
    fn test_provision_profile_with_named_envs() {
        let json_str = r#"{
            "runtime": {
                "platform": "avocado-raspberrypi4",
                "architecture": "arm64",
                "provision_default": "img"
            },
            "provision": {
                "envs": {
                    "device_info": {
                        "AVOCADO_DEVICE_CERT": "${AVOCADO_DEVICE_CERT}",
                        "AVOCADO_DEVICE_KEY": "${AVOCADO_DEVICE_KEY}",
                        "AVOCADO_DEVICE_ID": "${AVOCADO_DEVICE_ID}"
                    }
                },
                "profiles": {
                    "img": {
                        "script": "stone-provision-img.sh",
                        "envs": ["device_info"]
                    }
                }
            },
            "storage_devices": {}
        }"#;

        let manifest: Manifest = serde_json::from_str(json_str).unwrap();

        assert_eq!(manifest.runtime.provision_default, Some("img".to_string()));
        assert!(manifest.provision.is_some());

        let provision = manifest.provision.as_ref().unwrap();
        let profile = provision.profiles.get("img").unwrap();
        assert_eq!(profile.script, "stone-provision-img.sh");

        let resolved_envs = provision.resolve_envs(profile).unwrap();
        assert_eq!(resolved_envs.len(), 3);
        assert!(resolved_envs.contains_key("AVOCADO_DEVICE_CERT"));
        assert!(resolved_envs.contains_key("AVOCADO_DEVICE_KEY"));
        assert!(resolved_envs.contains_key("AVOCADO_DEVICE_ID"));
    }

    #[test]
    fn test_provision_profile_with_inline_envs() {
        let json_str = r#"{
            "runtime": {
                "platform": "avocado-raspberrypi4",
                "architecture": "arm64"
            },
            "provision": {
                "profiles": {
                    "test": {
                        "script": "test.sh",
                        "envs": [
                            {"INLINE_VAR": "value1"},
                            {"ANOTHER_VAR": "value2"}
                        ]
                    }
                }
            },
            "storage_devices": {}
        }"#;

        let manifest: Manifest = serde_json::from_str(json_str).unwrap();
        let provision = manifest.provision.as_ref().unwrap();
        let profile = provision.profiles.get("test").unwrap();

        let resolved_envs = provision.resolve_envs(profile).unwrap();
        assert_eq!(resolved_envs.len(), 2);
        assert_eq!(resolved_envs.get("INLINE_VAR"), Some(&"value1".to_string()));
        assert_eq!(
            resolved_envs.get("ANOTHER_VAR"),
            Some(&"value2".to_string())
        );
    }

    #[test]
    fn test_provision_profile_mixed_envs() {
        let json_str = r#"{
            "runtime": {
                "platform": "avocado-raspberrypi4",
                "architecture": "arm64"
            },
            "provision": {
                "envs": {
                    "base": {
                        "BASE_VAR": "base_value",
                        "OVERRIDE_ME": "original"
                    }
                },
                "profiles": {
                    "mixed": {
                        "script": "mixed.sh",
                        "envs": [
                            "base",
                            {"OVERRIDE_ME": "overridden", "INLINE_VAR": "inline_value"}
                        ]
                    }
                }
            },
            "storage_devices": {}
        }"#;

        let manifest: Manifest = serde_json::from_str(json_str).unwrap();
        let provision = manifest.provision.as_ref().unwrap();
        let profile = provision.profiles.get("mixed").unwrap();

        let resolved_envs = provision.resolve_envs(profile).unwrap();
        assert_eq!(resolved_envs.len(), 3);
        assert_eq!(
            resolved_envs.get("BASE_VAR"),
            Some(&"base_value".to_string())
        );
        assert_eq!(
            resolved_envs.get("OVERRIDE_ME"),
            Some(&"overridden".to_string())
        );
        assert_eq!(
            resolved_envs.get("INLINE_VAR"),
            Some(&"inline_value".to_string())
        );
    }

    #[test]
    fn test_provision_env_expansion() {
        unsafe {
            std::env::set_var("TEST_VAR", "expanded_value");
        }

        let provision = Provision {
            envs: None,
            files: vec![],
            profiles: HashMap::new(),
        };

        let mut test_envs = HashMap::new();
        test_envs.insert("NORMAL_VAR".to_string(), "normal".to_string());
        test_envs.insert("EXPANDED_VAR".to_string(), "${TEST_VAR}".to_string());
        test_envs.insert(
            "MIXED_VAR".to_string(),
            "prefix_${TEST_VAR}_suffix".to_string(),
        );

        let expanded = provision.expand_env_vars(&test_envs);

        assert_eq!(expanded.get("NORMAL_VAR"), Some(&"normal".to_string()));
        assert_eq!(
            expanded.get("EXPANDED_VAR"),
            Some(&"expanded_value".to_string())
        );
        assert_eq!(
            expanded.get("MIXED_VAR"),
            Some(&"prefix_expanded_value_suffix".to_string())
        );

        unsafe {
            std::env::remove_var("TEST_VAR");
        }
    }

    #[test]
    fn test_provision_env_expansion_undefined_vars() {
        let provision = Provision {
            envs: None,
            files: vec![],
            profiles: HashMap::new(),
        };

        let mut test_envs = HashMap::new();
        test_envs.insert(
            "UNDEFINED_VAR".to_string(),
            "${NONEXISTENT_VAR}".to_string(),
        );
        test_envs.insert(
            "MIXED_UNDEFINED".to_string(),
            "prefix_${ANOTHER_NONEXISTENT}_suffix".to_string(),
        );

        // Should replace undefined vars with empty string and warn (warning goes to stdout)
        let expanded = provision.expand_env_vars(&test_envs);

        assert_eq!(expanded.get("UNDEFINED_VAR"), Some(&"".to_string()));
        assert_eq!(
            expanded.get("MIXED_UNDEFINED"),
            Some(&"prefix__suffix".to_string())
        );
    }

    #[test]
    fn test_provision_env_expansion_multiple_vars_in_one_value() {
        unsafe {
            std::env::set_var("FIRST_VAR", "first");
            std::env::set_var("SECOND_VAR", "second");
        }

        let provision = Provision {
            envs: None,
            files: vec![],
            profiles: HashMap::new(),
        };

        let mut test_envs = HashMap::new();
        test_envs.insert(
            "MULTI_VAR".to_string(),
            "${FIRST_VAR}_middle_${SECOND_VAR}".to_string(),
        );

        let expanded = provision.expand_env_vars(&test_envs);

        assert_eq!(
            expanded.get("MULTI_VAR"),
            Some(&"first_middle_second".to_string())
        );

        unsafe {
            std::env::remove_var("FIRST_VAR");
            std::env::remove_var("SECOND_VAR");
        }
    }

    #[test]
    fn test_provision_env_expansion_empty_var() {
        unsafe {
            std::env::set_var("EMPTY_VAR", "");
        }

        let provision = Provision {
            envs: None,
            files: vec![],
            profiles: HashMap::new(),
        };

        let mut test_envs = HashMap::new();
        test_envs.insert("TEST".to_string(), "${EMPTY_VAR}".to_string());

        // Empty vars should also be replaced with empty string and trigger warning
        let expanded = provision.expand_env_vars(&test_envs);

        assert_eq!(expanded.get("TEST"), Some(&"".to_string()));

        unsafe {
            std::env::remove_var("EMPTY_VAR");
        }
    }

    #[test]
    fn test_manifest_get_provision_profile() {
        let json_str = r#"{
            "runtime": {
                "platform": "test",
                "architecture": "x86_64",
                "provision_default": "default_profile"
            },
            "provision": {
                "profiles": {
                    "profile1": {
                        "script": "script1.sh"
                    },
                    "profile2": {
                        "script": "script2.sh"
                    }
                }
            },
            "storage_devices": {}
        }"#;

        let manifest: Manifest = serde_json::from_str(json_str).unwrap();

        assert!(manifest.get_provision_profile("profile1").is_some());
        assert!(manifest.get_provision_profile("nonexistent").is_none());
        assert_eq!(manifest.get_provision_default(), Some("default_profile"));
    }

    // --- deep_merge_json tests ---

    #[test]
    fn test_deep_merge_scalar_override() {
        let mut base = serde_json::json!({"runtime": {"platform": "a", "architecture": "arm64"}});
        let overlay = serde_json::json!({"runtime": {"platform": "b"}});
        deep_merge_json(&mut base, overlay);
        assert_eq!(base["runtime"]["platform"], "b");
        assert_eq!(base["runtime"]["architecture"], "arm64");
    }

    #[test]
    fn test_deep_merge_adds_new_keys() {
        let mut base = serde_json::json!({"runtime": {"platform": "a"}});
        let overlay = serde_json::json!({"runtime": {"architecture": "arm64"}});
        deep_merge_json(&mut base, overlay);
        assert_eq!(base["runtime"]["platform"], "a");
        assert_eq!(base["runtime"]["architecture"], "arm64");
    }

    #[test]
    fn test_deep_merge_nested_objects() {
        let mut base = serde_json::json!({
            "storage_devices": {
                "rootdisk": {
                    "images": {
                        "boot": {"out": "boot.img", "size": 128}
                    }
                }
            }
        });
        let overlay = serde_json::json!({
            "storage_devices": {
                "rootdisk": {
                    "images": {
                        "rootfs": "rootfs.img"
                    }
                }
            }
        });
        deep_merge_json(&mut base, overlay);
        assert_eq!(base["storage_devices"]["rootdisk"]["images"]["boot"]["out"], "boot.img");
        assert_eq!(base["storage_devices"]["rootdisk"]["images"]["rootfs"], "rootfs.img");
    }

    #[test]
    fn test_deep_merge_named_array_merge_by_name() {
        let mut base = serde_json::json!({
            "partitions": [
                {"name": "boot", "size": 128, "size_unit": "mebibytes"},
                {"name": "var", "size": 512, "size_unit": "mebibytes", "expand": "true"}
            ]
        });
        let overlay = serde_json::json!({
            "partitions": [
                {"name": "var", "size": 1024}
            ]
        });
        deep_merge_json(&mut base, overlay);

        let partitions = base["partitions"].as_array().unwrap();
        assert_eq!(partitions.len(), 2);
        // boot unchanged
        assert_eq!(partitions[0]["name"], "boot");
        assert_eq!(partitions[0]["size"], 128);
        // var merged — size changed, other fields preserved
        assert_eq!(partitions[1]["name"], "var");
        assert_eq!(partitions[1]["size"], 1024);
        assert_eq!(partitions[1]["size_unit"], "mebibytes");
        assert_eq!(partitions[1]["expand"], "true");
    }

    #[test]
    fn test_deep_merge_named_array_appends_new() {
        let mut base = serde_json::json!({
            "partitions": [
                {"name": "boot", "size": 128, "size_unit": "mebibytes"}
            ]
        });
        let overlay = serde_json::json!({
            "partitions": [
                {"name": "data", "size": 2048, "size_unit": "mebibytes"}
            ]
        });
        deep_merge_json(&mut base, overlay);

        let partitions = base["partitions"].as_array().unwrap();
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0]["name"], "boot");
        assert_eq!(partitions[1]["name"], "data");
        assert_eq!(partitions[1]["size"], 2048);
    }

    #[test]
    fn test_deep_merge_named_array_preserves_unmatched() {
        let mut base = serde_json::json!({
            "partitions": [
                {"name": "boot", "size": 128, "size_unit": "mebibytes"},
                {"name": "rootfs", "size": 160, "size_unit": "mebibytes"},
                {"name": "var", "size": 512, "size_unit": "mebibytes"}
            ]
        });
        let overlay = serde_json::json!({
            "partitions": [
                {"name": "var", "size": 1024}
            ]
        });
        deep_merge_json(&mut base, overlay);

        let partitions = base["partitions"].as_array().unwrap();
        assert_eq!(partitions.len(), 3);
        assert_eq!(partitions[0]["name"], "boot");
        assert_eq!(partitions[0]["size"], 128);
        assert_eq!(partitions[1]["name"], "rootfs");
        assert_eq!(partitions[1]["size"], 160);
        assert_eq!(partitions[2]["name"], "var");
        assert_eq!(partitions[2]["size"], 1024);
    }

    #[test]
    fn test_deep_merge_unnamed_array_replaces() {
        let mut base = serde_json::json!({"files": ["a.txt", "b.txt", "c.txt"]});
        let overlay = serde_json::json!({"files": ["x.txt"]});
        deep_merge_json(&mut base, overlay);
        assert_eq!(base["files"], serde_json::json!(["x.txt"]));
    }

    #[test]
    fn test_deep_merge_empty_overlay() {
        let mut base = serde_json::json!({"runtime": {"platform": "test"}});
        let original = base.clone();
        deep_merge_json(&mut base, serde_json::json!({}));
        assert_eq!(base, original);
    }

    #[test]
    fn test_deep_merge_multiple_overlays_ordered() {
        let mut base = serde_json::json!({"runtime": {"platform": "original"}});
        deep_merge_json(
            &mut base,
            serde_json::json!({"runtime": {"platform": "first"}}),
        );
        deep_merge_json(
            &mut base,
            serde_json::json!({"runtime": {"platform": "second"}}),
        );
        assert_eq!(base["runtime"]["platform"], "second");
    }

    #[test]
    fn test_deep_merge_adds_top_level_section() {
        let mut base = serde_json::json!({
            "runtime": {"platform": "test", "architecture": "arm64"},
            "storage_devices": {}
        });
        let overlay = serde_json::json!({
            "provision": {
                "profiles": {
                    "img": {"script": "provision.sh"}
                }
            }
        });
        deep_merge_json(&mut base, overlay);
        assert_eq!(base["provision"]["profiles"]["img"]["script"], "provision.sh");
        assert_eq!(base["runtime"]["platform"], "test");
    }

    #[test]
    fn test_from_file_with_overlays_integration() {
        let dir = tempfile::tempdir().unwrap();

        let base_json = r#"{
            "runtime": {"platform": "base-platform", "architecture": "arm64"},
            "storage_devices": {
                "rootdisk": {
                    "out": "disk.img",
                    "devpath": "/dev/mmcblk0",
                    "images": {},
                    "partitions": [
                        {"name": "boot", "size": 128, "size_unit": "mebibytes"},
                        {"name": "var", "size": 512, "size_unit": "mebibytes"}
                    ]
                }
            }
        }"#;
        let overlay_json = r#"{
            "runtime": {"platform": "overlay-platform"},
            "storage_devices": {
                "rootdisk": {
                    "partitions": [
                        {"name": "var", "size": 1024}
                    ]
                }
            }
        }"#;

        let base_path = dir.path().join("base.json");
        let overlay_path = dir.path().join("overlay.json");
        std::fs::write(&base_path, base_json).unwrap();
        std::fs::write(&overlay_path, overlay_json).unwrap();

        let manifest =
            Manifest::from_file_with_overlays(&base_path, &[overlay_path]).unwrap();

        assert_eq!(manifest.runtime.platform, "overlay-platform");
        assert_eq!(manifest.runtime.architecture, "arm64");

        let rootdisk = manifest.storage_devices.get("rootdisk").unwrap();
        assert_eq!(rootdisk.partitions.len(), 2);
        assert_eq!(rootdisk.partitions[0].name, Some("boot".to_string()));
        assert_eq!(rootdisk.partitions[0].size, 128);
        assert_eq!(rootdisk.partitions[1].name, Some("var".to_string()));
        assert_eq!(rootdisk.partitions[1].size, 1024);
    }

    #[test]
    fn test_from_file_with_overlays_bad_json() {
        let dir = tempfile::tempdir().unwrap();

        let base_path = dir.path().join("base.json");
        let overlay_path = dir.path().join("bad.json");
        std::fs::write(
            &base_path,
            r#"{"runtime":{"platform":"x","architecture":"y"},"storage_devices":{}}"#,
        )
        .unwrap();
        std::fs::write(&overlay_path, "not valid json{{{").unwrap();

        let err = Manifest::from_file_with_overlays(&base_path, &[overlay_path.clone()])
            .unwrap_err();
        assert!(err.contains("Failed to parse overlay JSON"));
        assert!(err.contains("bad.json"));
    }

    #[test]
    fn test_from_file_with_overlays_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().join("base.json");
        std::fs::write(
            &base_path,
            r#"{"runtime":{"platform":"x","architecture":"y"},"storage_devices":{}}"#,
        )
        .unwrap();

        let overlay_path = dir.path().join("nonexistent.json");
        let err = Manifest::from_file_with_overlays(&base_path, &[overlay_path]).unwrap_err();
        assert!(err.contains("Failed to read overlay file"));
    }

    #[test]
    fn test_load_and_merge_returns_json_string() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().join("base.json");
        let overlay_path = dir.path().join("overlay.json");
        std::fs::write(
            &base_path,
            r#"{"runtime":{"platform":"a","architecture":"arm64"},"storage_devices":{}}"#,
        )
        .unwrap();
        std::fs::write(&overlay_path, r#"{"runtime":{"platform":"b"}}"#).unwrap();

        let (manifest, merged_json) =
            Manifest::load_and_merge(&base_path, &[overlay_path]).unwrap();
        assert_eq!(manifest.runtime.platform, "b");
        let json_str = merged_json.unwrap();
        assert!(json_str.contains("\"platform\": \"b\""));
    }

    #[test]
    fn test_load_and_merge_no_overlays_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().join("base.json");
        std::fs::write(
            &base_path,
            r#"{"runtime":{"platform":"a","architecture":"arm64"},"storage_devices":{}}"#,
        )
        .unwrap();

        let (_, merged_json) = Manifest::load_and_merge(&base_path, &[]).unwrap();
        assert!(merged_json.is_none());
    }
}
