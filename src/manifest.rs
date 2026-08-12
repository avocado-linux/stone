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

/// Unknown keys are refused rather than ignored. `files_append` is the first key
/// here whose author is a delivery hook rather than a person, so a misspelling or
/// a key nested one level off produced an image without the overlay, exited 0,
/// and left a missing line in `describe-manifest` as the only signal - by which
/// point the board has already booted without its device-tree overlay.
///
/// The attribute sits on the enum, not on the variant: `deny_unknown_fields` is a
/// container attribute, and serde's derive knows to exempt the `type`
/// discriminator that internal tagging puts in the same map.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum BuildArgs {
    #[serde(rename = "fat")]
    Fat {
        variant: FatVariant,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        files: Vec<FileEntry>,
        /// Additional FAT files merged onto `files` at bundle time, deduplicated
        /// by output path. Lets a delivery hook append a custom entry (e.g. a
        /// device-tree overlay) without restating the base `files` list.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        files_append: Vec<FileEntry>,
        /// FAT volume label. `src/fat.rs` has carried the write for this all
        /// along; there was no field here to plumb it from, so three shipped
        /// manifests asking for `BOOT` produced images labelled with the `FATFS`
        /// default instead - silently, which is what refusing unknown keys now
        /// prevents.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
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

    #[allow(dead_code)]
    pub fn fat_files_append(&self) -> &[FileEntry] {
        match self {
            BuildArgs::Fat { files_append, .. } => files_append,
            _ => &[],
        }
    }

    #[allow(dead_code)]
    pub fn fat_label(&self) -> Option<&str> {
        match self {
            BuildArgs::Fat { label, .. } => label.as_deref(),
            _ => None,
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

#[derive(Debug, Serialize)]
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

/// The object form of [`Image`], as a named struct so it has a derived
/// deserializer of its own to delegate to.
///
/// Exists only for [`Image`]'s `Deserialize` impl. Field drift is caught by the
/// compiler: `Image::Object` is constructed exhaustively from this struct, so a
/// field added to the variant fails to compile until it is added here too.
#[derive(Deserialize)]
struct ImageObject {
    out: String,
    #[serde(default)]
    build_args: Option<BuildArgs>,
    size: i64,
    size_unit: String,
    #[serde(default)]
    block_size: Option<u32>,
    #[serde(default)]
    uuid: Option<String>,
}

/// Hand-written rather than `#[serde(untagged)]` so a bad object reports which
/// key was wrong.
///
/// `untagged` discards every variant's error when all of them fail, so the
/// `unknown field ... expected one of ...` that `deny_unknown_fields` produces
/// inside `BuildArgs` never reached the operator - what surfaced was `data did
/// not match any variant of untagged enum Image`, positioned at the end of the
/// enclosing object rather than at the offending key. Refusing an unknown key is
/// only useful if the message names it.
///
/// Dispatch here is unambiguous - a string is the `String` form, anything else
/// must be the object form - so the object branch's own error can be propagated
/// verbatim instead of guessed at.
impl<'de> Deserialize<'de> for Image {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if let Value::String(filename) = value {
            return Ok(Image::String(filename));
        }
        let object: ImageObject =
            serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Image::Object {
            out: object.out,
            build_args: object.build_args,
            size: object.size,
            size_unit: object.size_unit,
            block_size: object.block_size,
            uuid: object.uuid,
        })
    }
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

    /// Every FAT file this image contributes: base `files` with `files_append`
    /// merged in, deduplicated by output path.
    ///
    /// [`Image::files`] returns only the base list, which is what left the
    /// feature half-wired: each consumer that stages source files copied base
    /// inputs and skipped appended ones, so `bundle` produced an `.aos` naming
    /// a file it had not staged and `provision` failed a command later. Anything
    /// that needs the files this image will actually contain wants this, not
    /// `files()`.
    ///
    /// Returns the conflict error from [`merge_fat_files`] rather than
    /// swallowing it, so a colliding append fails at the caller instead of
    /// silently losing one of the two entries.
    pub fn all_files(&self) -> Result<Vec<FileEntry>, String> {
        match self.build_args() {
            Some(args) => merge_fat_files(args.fat_files(), args.fat_files_append()),
            None => Ok(self.files().to_vec()),
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

#[derive(Debug, Clone, Deserialize, Serialize)]
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

    /// Output path this entry produces on the target filesystem. A bare
    /// `String` entry outputs under its own name; an `Object` outputs under
    /// its explicit `out`.
    pub fn output_name(&self) -> &str {
        match self {
            FileEntry::String(filename) => filename,
            FileEntry::Object { output, .. } => output,
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
    /// May be omitted only on the last partition when `expand: "true"`.
    /// In that case the size is supplied at bundle/provision time via
    /// `--partition-size <name>=<bytes>` and rounded up to size_alignment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expand: Option<String>,
    /// Alignment boundary applied to an externally-supplied size when `size`
    /// is omitted. Defaults to 4 mebibytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_alignment: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_alignment_unit: Option<String>,
}

/// Convert a size value to bytes based on its unit. SI prefixes use 1000-base,
/// IEC prefixes use 1024-base. Unknown units return the raw value.
pub fn to_bytes(value: u64, unit: Option<&str>) -> u64 {
    match unit {
        Some("tebibytes") => value * 1024 * 1024 * 1024 * 1024,
        Some("gibibytes") => value * 1024 * 1024 * 1024,
        Some("mebibytes") => value * 1024 * 1024,
        Some("kibibytes") => value * 1024,
        Some("terabytes") => value * 1_000_000_000_000,
        Some("gigabytes") => value * 1_000_000_000,
        Some("megabytes") => value * 1_000_000,
        Some("kilobytes") => value * 1_000,
        Some("bytes") | None => value,
        _ => value,
    }
}

/// Round `value` up to the next multiple of `alignment`. Returns `value`
/// unchanged when alignment is 0.
pub fn align_up(value: u64, alignment: u64) -> u64 {
    if alignment == 0 {
        value
    } else {
        value.div_ceil(alignment) * alignment
    }
}

/// Effective alignment (bytes) for a partition's externally-supplied size.
/// Defaults to 4 MiB when `size_alignment` is unspecified.
pub fn partition_alignment_bytes(p: &Partition) -> u64 {
    let val = p.size_alignment.unwrap_or(4) as u64;
    let unit = p.size_alignment_unit.as_deref().unwrap_or("mebibytes");
    to_bytes(val, Some(unit))
}

/// Resolve a partition's size to bytes, consulting the external override map
/// when the manifest omits `size`. Returns the byte size and a unit hint
/// (`"bytes"` when the override path was taken).
pub fn resolve_partition_size_bytes(
    p: &Partition,
    overrides: &HashMap<String, u64>,
) -> Result<(u64, String), String> {
    if let Some(size) = p.size {
        let unit = p
            .size_unit
            .as_deref()
            .ok_or_else(|| "partition has size but no size_unit".to_string())?;
        return Ok((to_bytes(size as u64, Some(unit)), unit.to_string()));
    }
    let name = p.name.as_deref().ok_or_else(|| {
        "partition omits size but has no name to match against overrides".to_string()
    })?;
    let raw = overrides.get(name).copied().ok_or_else(|| {
        format!("partition '{name}' omits size; no --partition-size override was supplied")
    })?;
    let aligned = align_up(raw, partition_alignment_bytes(p));
    Ok((aligned, "bytes".to_string()))
}

impl Manifest {
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read manifest file '{}': {}", path.display(), e))?;

        let manifest: Self = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse manifest JSON '{}': {}", path.display(), e))?;
        manifest.validate_partitions().map_err(|e| {
            format!(
                "Manifest '{}' has invalid partition layout: {}",
                path.display(),
                e
            )
        })?;
        Ok(manifest)
    }

    /// Enforce structural rules on partition lists that serde alone can't express:
    /// - if `size` is omitted, the partition must be the last in its device and
    ///   carry `expand: "true"`.
    /// - `size` and `size_unit` must both be present or both be absent.
    pub fn validate_partitions(&self) -> Result<(), String> {
        for (dev_name, device) in &self.storage_devices {
            let last_idx = device.partitions.len().saturating_sub(1);
            for (idx, p) in device.partitions.iter().enumerate() {
                let label = p.name.clone().unwrap_or_else(|| format!("#{idx}"));
                match (p.size.is_some(), p.size_unit.is_some()) {
                    (true, false) => {
                        return Err(format!(
                            "device '{dev_name}' partition '{label}' has size but no size_unit",
                        ));
                    }
                    (false, true) => {
                        return Err(format!(
                            "device '{dev_name}' partition '{label}' has size_unit but no size",
                        ));
                    }
                    (false, false) => {
                        if idx != last_idx {
                            return Err(format!(
                                "device '{dev_name}' partition '{label}' may only omit size if it is the last partition in the device's partition list"
                            ));
                        }
                        if p.expand.as_deref() != Some("true") {
                            return Err(format!(
                                "device '{dev_name}' partition '{label}' may only omit size if it has expand=\"true\""
                            ));
                        }
                        if p.name.is_none() {
                            return Err(format!(
                                "device '{dev_name}' partition at index {idx} omits size and has no name; a name is required so a --partition-size override can target it"
                            ));
                        }
                    }
                    (true, true) => {}
                }
            }
        }
        Ok(())
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
                "Failed to read manifest file '{}': {}",
                base_path.display(),
                e
            )
        })?;
        let mut merged: Value = serde_json::from_str(&base_content).map_err(|e| {
            format!(
                "Failed to parse manifest JSON '{}': {}",
                base_path.display(),
                e
            )
        })?;

        for overlay_path in overlay_paths {
            let overlay_content = std::fs::read_to_string(overlay_path).map_err(|e| {
                format!(
                    "Failed to read overlay file '{}': {}",
                    overlay_path.display(),
                    e
                )
            })?;
            let overlay_value: Value = serde_json::from_str(&overlay_content).map_err(|e| {
                format!(
                    "Failed to parse overlay JSON '{}': {}",
                    overlay_path.display(),
                    e
                )
            })?;
            deep_merge_json(&mut merged, overlay_value);
        }

        let merged_json = serde_json::to_string_pretty(&merged)
            .map_err(|e| format!("Failed to serialize merged manifest: {}", e))?;

        let manifest: Self = serde_json::from_value(merged).map_err(|e| {
            format!(
                "Merged manifest (base '{}' + {} overlay(s)) is invalid: {}",
                base_path.display(),
                overlay_paths.len(),
                e
            )
        })?;
        manifest.validate_partitions().map_err(|e| {
            format!(
                "Merged manifest (base '{}' + {} overlay(s)) has invalid partition layout: {}",
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
                                    "Named environment block '{env_name}' not found in provision.envs."
                                ));
                            }
                        } else {
                            return Err(format!(
                                "Named environment block '{env_name}' referenced but no provision.envs defined."
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
/// Concatenate `overlay` onto `base` when both are arrays, rather than
/// replacing. Falls back to replacement for any other shape, matching
/// [`deep_merge_json`]'s behaviour on a type mismatch.
fn concat_json_array(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Array(base_arr), Value::Array(overlay_arr)) => base_arr.extend(overlay_arr),
        (base, overlay) => *base = overlay,
    }
}

pub fn deep_merge_json(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_val) in overlay_map {
                let is_append_list = key == "files_append";
                let entry = base_map.entry(key).or_insert(Value::Null);
                if is_append_list {
                    // `files_append` is additive by definition - its purpose is
                    // letting independent delivery hooks each contribute an
                    // entry without restating the others. The generic array rule
                    // below replaces the list unless every element carries a
                    // "name", and FileEntry is {in,out}, so routing this through
                    // it made the second overlay silently drop the first: the
                    // key inherited the exact clobbering it exists to avoid.
                    //
                    // Concatenating rather than deduplicating here on purpose.
                    // merge_fat_files owns that policy, and it distinguishes an
                    // idempotent re-emit from a real collision; doing it twice,
                    // in two places, is how the two would drift.
                    concat_json_array(entry, overlay_val);
                } else {
                    deep_merge_json(entry, overlay_val);
                }
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

/// Merge a base FAT `files` list with an additive `files_append` list, keyed by
/// output path. An entry whose `{in,out}` is identical to one already present
/// collapses to a single entry, so a regenerative delivery hook that re-emits
/// the same entry on every rebuild is an idempotent no-op. Two entries that
/// share an output path but resolve from different inputs are a hard error: a
/// silent overwrite of a boot file (e.g. `overlays/foo.dtbo`) could brick the
/// device, so the conflict must surface at build time.
///
/// Only appended entries are checked. Base `files` are carried through
/// unexamined even when two of them share an output: such a manifest built
/// before this feature existed (fatfs is last-write-wins) and turning it into a
/// build failure here would be an unannounced break of manifests nobody
/// touched. The dedup is a property of appending, which is what the doc above
/// describes and what a delivery hook can actually trip.
///
/// Comparison is on [`fat_output_key`], not the raw string - see there for why
/// exact equality was not enough.
pub fn merge_fat_files(base: &[FileEntry], append: &[FileEntry]) -> Result<Vec<FileEntry>, String> {
    let mut merged: Vec<FileEntry> = base.to_vec();
    for entry in append {
        if let Some(existing) = merged
            .iter()
            .find(|e| fat_output_key(e.output_name()) == fat_output_key(entry.output_name()))
        {
            if existing.input_filename() != entry.input_filename() {
                return Err(format!(
                    "Conflicting FAT file entries for output '{}': inputs '{}' and '{}' differ. \
A custom overlay must not overwrite an existing boot file; give it a distinct output name.",
                    entry.output_name(),
                    existing.input_filename(),
                    entry.input_filename(),
                ));
            }
            // Identical {in,out}: idempotent, already present.
            continue;
        }
        merged.push(entry.clone());
    }
    Ok(merged)
}

/// Normalize a FAT output path to the identity the filesystem will actually
/// resolve it by.
///
/// Comparing output paths verbatim let a variant duplicate through the guard
/// and into the image, where fatfs collapses it anyway: `eq_name` uppercases
/// both sides, `find_entry` compares ignoring case, and `create_file` returns
/// the existing entry *without truncating*. So `overlays/VC4.DTBO` written over
/// `overlays/vc4.dtbo` produced one directory entry holding the overlay's bytes
/// followed by whatever of the original ran past them - the brick this guard
/// exists to prevent, reached through a spelling it did not check.
///
/// Uppercasing covers the case half. Dropping empty segments covers the path
/// form half, so `./x`, `x` and `a//b` do not read as three distinct outputs.
fn fat_output_key(output: &str) -> String {
    output
        .split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
        .map(|segment| segment.to_uppercase())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_args_serialization() {
        let fat_args = BuildArgs::Fat {
            variant: FatVariant::Fat32,
            files: vec![],
            files_append: vec![],
            label: None,
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
            files_append: vec![],
            label: None,
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
                files_append: vec![],
                label: None,
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
            files_append: vec![],
            label: None,
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
        assert_eq!(partition.size, Some(128));
        assert_eq!(partition.size_unit, Some("kibibytes".to_string()));
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
        assert_eq!(
            base["storage_devices"]["rootdisk"]["images"]["boot"]["out"],
            "boot.img"
        );
        assert_eq!(
            base["storage_devices"]["rootdisk"]["images"]["rootfs"],
            "rootfs.img"
        );
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
        assert_eq!(
            base["provision"]["profiles"]["img"]["script"],
            "provision.sh"
        );
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

        let manifest = Manifest::from_file_with_overlays(&base_path, &[overlay_path]).unwrap();

        assert_eq!(manifest.runtime.platform, "overlay-platform");
        assert_eq!(manifest.runtime.architecture, "arm64");

        let rootdisk = manifest.storage_devices.get("rootdisk").unwrap();
        assert_eq!(rootdisk.partitions.len(), 2);
        assert_eq!(rootdisk.partitions[0].name, Some("boot".to_string()));
        assert_eq!(rootdisk.partitions[0].size, Some(128));
        assert_eq!(rootdisk.partitions[1].name, Some("var".to_string()));
        assert_eq!(rootdisk.partitions[1].size, Some(1024));
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

        let err =
            Manifest::from_file_with_overlays(&base_path, std::slice::from_ref(&overlay_path))
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

    fn partition_with_size(
        size: Option<i64>,
        size_unit: Option<&str>,
        expand: Option<&str>,
        name: Option<&str>,
    ) -> Partition {
        Partition {
            name: name.map(String::from),
            image: None,
            partition_type: None,
            partition_uuid: None,
            offset: None,
            offset_unit: None,
            offset_redundant: None,
            offset_redundant_unit: None,
            size,
            size_unit: size_unit.map(String::from),
            expand: expand.map(String::from),
            size_alignment: None,
            size_alignment_unit: None,
        }
    }

    fn manifest_with_partitions(partitions: Vec<Partition>) -> Manifest {
        let mut storage_devices = HashMap::new();
        storage_devices.insert(
            "main".to_string(),
            StorageDevice {
                out: "disk.img".to_string(),
                build_args: None,
                devpath: "/dev/sda".to_string(),
                block_size: None,
                uuid: None,
                images: HashMap::new(),
                partitions,
            },
        );
        Manifest {
            runtime: Runtime {
                platform: "linux".to_string(),
                architecture: "x86_64".to_string(),
                provision: None,
                provision_default: None,
                update_strategy: None,
            },
            storage_devices,
            provision: None,
            update: None,
        }
    }

    #[test]
    fn test_to_bytes_units() {
        assert_eq!(to_bytes(5, Some("bytes")), 5);
        assert_eq!(to_bytes(2, Some("kibibytes")), 2 * 1024);
        assert_eq!(to_bytes(3, Some("mebibytes")), 3 * 1024 * 1024);
        assert_eq!(to_bytes(4, Some("gibibytes")), 4u64 * 1024 * 1024 * 1024);
        assert_eq!(to_bytes(2, Some("kilobytes")), 2_000);
        assert_eq!(to_bytes(7, None), 7);
        assert_eq!(to_bytes(9, Some("weird")), 9);
    }

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0, 4096), 0);
        assert_eq!(align_up(1, 4096), 4096);
        assert_eq!(align_up(4096, 4096), 4096);
        assert_eq!(align_up(4097, 4096), 8192);
        assert_eq!(align_up(100, 0), 100);
    }

    #[test]
    fn test_partition_alignment_bytes_default_is_4_mib() {
        let p = partition_with_size(None, None, Some("true"), Some("var"));
        assert_eq!(partition_alignment_bytes(&p), 4 * 1024 * 1024);
    }

    #[test]
    fn test_partition_alignment_bytes_explicit() {
        let mut p = partition_with_size(None, None, Some("true"), Some("var"));
        p.size_alignment = Some(16);
        p.size_alignment_unit = Some("mebibytes".to_string());
        assert_eq!(partition_alignment_bytes(&p), 16 * 1024 * 1024);
    }

    #[test]
    fn test_resolve_partition_size_uses_explicit_size() {
        let p = partition_with_size(Some(128), Some("mebibytes"), None, Some("rootfs"));
        let overrides = HashMap::new();
        let (bytes, unit) = resolve_partition_size_bytes(&p, &overrides).unwrap();
        assert_eq!(bytes, 128 * 1024 * 1024);
        assert_eq!(unit, "mebibytes");
    }

    #[test]
    fn test_resolve_partition_size_uses_override_and_aligns() {
        let p = partition_with_size(None, None, Some("true"), Some("var"));
        // 100 MiB raw -> default 4 MiB alignment -> 100 MiB is already aligned
        let mut overrides = HashMap::new();
        overrides.insert("var".to_string(), 100 * 1024 * 1024);
        let (bytes, unit) = resolve_partition_size_bytes(&p, &overrides).unwrap();
        assert_eq!(bytes, 100 * 1024 * 1024);
        assert_eq!(unit, "bytes");

        // 100 MiB + 1 byte -> rounds up to next 4 MiB boundary = 104 MiB
        let mut overrides2 = HashMap::new();
        overrides2.insert("var".to_string(), 100 * 1024 * 1024 + 1);
        let (bytes2, _) = resolve_partition_size_bytes(&p, &overrides2).unwrap();
        assert_eq!(bytes2, 104 * 1024 * 1024);
    }

    #[test]
    fn test_resolve_partition_size_custom_alignment() {
        let mut p = partition_with_size(None, None, Some("true"), Some("var"));
        p.size_alignment = Some(16);
        p.size_alignment_unit = Some("mebibytes".to_string());
        let mut overrides = HashMap::new();
        // 100 MiB rounds up to next 16 MiB = 112 MiB
        overrides.insert("var".to_string(), 100 * 1024 * 1024);
        let (bytes, _) = resolve_partition_size_bytes(&p, &overrides).unwrap();
        assert_eq!(bytes, 112 * 1024 * 1024);
    }

    #[test]
    fn test_resolve_partition_size_missing_override_errors() {
        let p = partition_with_size(None, None, Some("true"), Some("var"));
        let overrides = HashMap::new();
        let err = resolve_partition_size_bytes(&p, &overrides).unwrap_err();
        assert!(
            err.contains("var"),
            "error should name the partition: {err}"
        );
        assert!(err.contains("--partition-size"));
    }

    #[test]
    fn test_validate_partitions_explicit_sizes_ok() {
        let m = manifest_with_partitions(vec![
            partition_with_size(Some(256), Some("mebibytes"), None, Some("boot")),
            partition_with_size(Some(512), Some("mebibytes"), Some("true"), Some("var")),
        ]);
        assert!(m.validate_partitions().is_ok());
    }

    #[test]
    fn test_validate_partitions_omitted_on_last_expand_ok() {
        let m = manifest_with_partitions(vec![
            partition_with_size(Some(256), Some("mebibytes"), None, Some("boot")),
            partition_with_size(None, None, Some("true"), Some("var")),
        ]);
        assert!(
            m.validate_partitions().is_ok(),
            "{:?}",
            m.validate_partitions()
        );
    }

    #[test]
    fn test_validate_partitions_omitted_on_non_last_fails() {
        let m = manifest_with_partitions(vec![
            partition_with_size(None, None, Some("true"), Some("first")),
            partition_with_size(Some(256), Some("mebibytes"), None, Some("second")),
        ]);
        let err = m.validate_partitions().unwrap_err();
        assert!(err.contains("first"), "{err}");
        assert!(err.contains("last partition"));
    }

    #[test]
    fn test_validate_partitions_omitted_without_expand_fails() {
        let m = manifest_with_partitions(vec![
            partition_with_size(Some(256), Some("mebibytes"), None, Some("boot")),
            partition_with_size(None, None, None, Some("var")),
        ]);
        let err = m.validate_partitions().unwrap_err();
        assert!(err.contains("expand"), "{err}");
    }

    #[test]
    fn test_validate_partitions_half_specified_size_fails() {
        let m_size_only = manifest_with_partitions(vec![Partition {
            name: Some("boot".to_string()),
            image: None,
            partition_type: None,
            partition_uuid: None,
            offset: None,
            offset_unit: None,
            offset_redundant: None,
            offset_redundant_unit: None,
            size: Some(100),
            size_unit: None,
            expand: None,
            size_alignment: None,
            size_alignment_unit: None,
        }]);
        let err = m_size_only.validate_partitions().unwrap_err();
        assert!(err.contains("size_unit"), "{err}");

        let m_unit_only = manifest_with_partitions(vec![Partition {
            name: Some("boot".to_string()),
            image: None,
            partition_type: None,
            partition_uuid: None,
            offset: None,
            offset_unit: None,
            offset_redundant: None,
            offset_redundant_unit: None,
            size: None,
            size_unit: Some("mebibytes".to_string()),
            expand: None,
            size_alignment: None,
            size_alignment_unit: None,
        }]);
        let err = m_unit_only.validate_partitions().unwrap_err();
        assert!(err.contains("size_unit"), "{err}");
    }

    #[test]
    fn test_validate_partitions_omitted_without_name_fails() {
        let m = manifest_with_partitions(vec![
            partition_with_size(Some(256), Some("mebibytes"), None, Some("boot")),
            partition_with_size(None, None, Some("true"), None),
        ]);
        let err = m.validate_partitions().unwrap_err();
        assert!(err.contains("name"), "{err}");
    }

    #[test]
    fn test_parse_manifest_with_omitted_size_partition() {
        let json = r#"{
            "runtime": {"platform": "linux", "architecture": "x86_64"},
            "storage_devices": {
                "main": {
                    "out": "disk.img",
                    "devpath": "/dev/sda",
                    "images": {},
                    "partitions": [
                        {"name": "boot", "size": 256, "size_unit": "mebibytes"},
                        {"name": "var", "expand": "true"}
                    ]
                }
            }
        }"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.json");
        std::fs::write(&path, json).unwrap();
        let m = Manifest::from_file(&path)
            .expect("from_file should accept omitted size with expand=true on last partition");
        let parts = &m.storage_devices["main"].partitions;
        assert_eq!(parts.len(), 2);
        assert!(parts[1].size.is_none());
        assert!(parts[1].size_unit.is_none());
    }

    // --- merge_fat_files (files_append) tests ---

    fn fe_obj(input: &str, output: &str) -> FileEntry {
        FileEntry::Object {
            input: input.to_string(),
            output: output.to_string(),
        }
    }

    #[test]
    fn test_file_entry_output_name_string_and_object() {
        // A bare String entry outputs under its own name (in == out).
        assert_eq!(
            FileEntry::String("a.dtbo".to_string()).output_name(),
            "a.dtbo"
        );
        // An Object entry outputs under its explicit `out`.
        assert_eq!(
            fe_obj("a.dtbo", "overlays/a.dtbo").output_name(),
            "overlays/a.dtbo"
        );
    }

    #[test]
    fn test_merge_fat_files_appends_new_entry() {
        let base = vec![fe_obj("config.txt", "config.txt")];
        let append = vec![fe_obj("bbproto.dtbo", "overlays/bbproto.dtbo")];
        let merged = merge_fat_files(&base, &append).unwrap();
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].output_name(), "config.txt");
        assert_eq!(merged[1].output_name(), "overlays/bbproto.dtbo");
    }

    #[test]
    fn test_merge_fat_files_identical_entry_collapses() {
        // A regenerative hook re-emits the same {in,out} on every rebuild; that
        // must be an idempotent no-op, not a hard error (Fable C2).
        let base = vec![fe_obj("bbproto.dtbo", "overlays/bbproto.dtbo")];
        let append = vec![fe_obj("bbproto.dtbo", "overlays/bbproto.dtbo")];
        let merged = merge_fat_files(&base, &append).unwrap();
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].output_name(), "overlays/bbproto.dtbo");
    }

    #[test]
    fn test_merge_fat_files_same_out_different_in_errors() {
        // Same output path, different source = a real conflict (would silently
        // overwrite a boot-critical file). Hard error (Fable C2 / Risk #5).
        let base = vec![fe_obj("vc4-kms-v3d-pi5.dtbo", "overlays/foo.dtbo")];
        let append = vec![fe_obj("user.dtbo", "overlays/foo.dtbo")];
        let err = merge_fat_files(&base, &append).unwrap_err();
        assert!(
            err.contains("overlays/foo.dtbo"),
            "error should name the colliding out: {err}"
        );
    }

    #[test]
    fn test_merge_fat_files_string_object_same_out_collapses() {
        // String("f") and Object{in:"f", out:"f"} describe the same delivery.
        let base = vec![FileEntry::String("f".to_string())];
        let append = vec![fe_obj("f", "f")];
        let merged = merge_fat_files(&base, &append).unwrap();
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn test_fat_build_args_parses_files_append() {
        let json = r#"{"type":"fat","variant":"FAT32","files":[{"in":"config.txt","out":"config.txt"}],"files_append":[{"in":"bbproto.dtbo","out":"overlays/bbproto.dtbo"}]}"#;
        let args: BuildArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.fat_files().len(), 1);
        assert_eq!(args.fat_files_append().len(), 1);
        assert_eq!(
            args.fat_files_append()[0].output_name(),
            "overlays/bbproto.dtbo"
        );
    }

    #[test]
    fn test_fat_build_args_files_append_defaults_empty() {
        // A manifest without files_append still parses (backward compatible).
        let json =
            r#"{"type":"fat","variant":"FAT32","files":[{"in":"config.txt","out":"config.txt"}]}"#;
        let args: BuildArgs = serde_json::from_str(json).unwrap();
        assert!(args.fat_files_append().is_empty());
    }

    #[test]
    fn two_overlays_each_appending_a_file_keep_both() {
        // The composability case, and the reason files_append exists: two
        // delivery hooks each contribute one overlay. Under the generic array
        // rule the second replaced the first and one .dtbo vanished with no
        // warning, so the collision guard never even saw it.
        let mut base = serde_json::json!({
            "build_args": {
                "type": "fat",
                "files_append": [{"in": "a.dtbo", "out": "overlays/a.dtbo"}]
            }
        });
        let overlay = serde_json::json!({
            "build_args": {
                "files_append": [{"in": "b.dtbo", "out": "overlays/b.dtbo"}]
            }
        });

        deep_merge_json(&mut base, overlay);

        let appended = base["build_args"]["files_append"].as_array().unwrap();
        assert_eq!(appended.len(), 2, "both overlays must survive the merge");
        assert_eq!(appended[0]["in"], "a.dtbo");
        assert_eq!(appended[1]["in"], "b.dtbo");
    }

    #[test]
    fn an_overlay_still_replaces_the_base_files_list() {
        // The other half of the same decision. `files` keeps replace semantics
        // - that is precisely why files_append had to exist - so appending must
        // not leak into it.
        let mut base = serde_json::json!({
            "build_args": {"files": [{"in": "old.bin", "out": "old.bin"}]}
        });
        let overlay = serde_json::json!({
            "build_args": {"files": [{"in": "new.bin", "out": "new.bin"}]}
        });

        deep_merge_json(&mut base, overlay);

        let files = base["build_args"]["files"].as_array().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0]["in"], "new.bin");
    }

    #[test]
    fn a_case_variant_output_is_a_collision_not_a_second_file() {
        // fatfs resolves names case-insensitively and create_file does not
        // truncate, so these two would have become one entry holding the
        // overlay's bytes followed by the tail of the original.
        let base = vec![FileEntry::Object {
            input: "vc4-kms-v3d-pi5.dtbo".to_string(),
            output: "overlays/vc4-kms-v3d-pi5.dtbo".to_string(),
        }];
        let append = vec![FileEntry::Object {
            input: "user.dtbo".to_string(),
            output: "overlays/VC4-KMS-V3D-PI5.DTBO".to_string(),
        }];

        let err = merge_fat_files(&base, &append).unwrap_err();
        assert!(err.contains("Conflicting FAT file entries"), "{err}");
        assert!(
            !err.starts_with("[ERROR]"),
            "the caller's log_error adds that prefix; carrying it here printed it twice: {err}"
        );
    }

    #[test]
    fn path_form_variants_of_one_output_collide() {
        let base = vec![FileEntry::Object {
            input: "a.bin".to_string(),
            output: "./boot//x.bin".to_string(),
        }];
        let append = vec![FileEntry::Object {
            input: "b.bin".to_string(),
            output: "boot/x.bin".to_string(),
        }];

        assert!(merge_fat_files(&base, &append).is_err());
    }

    #[test]
    fn duplicate_outputs_within_base_files_are_not_a_collision() {
        // Pre-existing manifests did this and built; the dedup is a property of
        // appending, so turning base-vs-base into a hard error would break
        // manifests nobody edited.
        let base = vec![
            FileEntry::Object {
                input: "a.bin".to_string(),
                output: "shared".to_string(),
            },
            FileEntry::Object {
                input: "b.bin".to_string(),
                output: "shared".to_string(),
            },
        ];

        let merged = merge_fat_files(&base, &[]).unwrap();
        assert_eq!(merged.len(), 2, "base entries pass through unexamined");
    }

    // --- Image deserialization diagnostics ---

    const IMAGE_WITH_BAD_BUILD_ARGS: &str = r#"{
        "out": "boot.img", "size": 64, "size_unit": "mebibytes",
        "build_args": {"type": "fat", "variant": "FAT32", "file_append": []}
    }"#;

    #[test]
    fn an_unknown_build_args_key_surfaces_through_image() {
        // `deny_unknown_fields` on BuildArgs produces a message naming the key and
        // listing the valid ones. Under `#[serde(untagged)]` on Image, serde threw it
        // away and reported "did not match any variant of untagged enum Image", so the
        // refusal told the operator nothing about what to change. This asserts the
        // inner message reaches the caller.
        let err = serde_json::from_str::<Image>(IMAGE_WITH_BAD_BUILD_ARGS).unwrap_err();
        let err = err.to_string();
        assert!(err.contains("file_append"), "must name the bad key: {err}");
        assert!(
            err.contains("files_append"),
            "must list the valid keys so the fix is obvious: {err}"
        );
        assert!(
            !err.contains("did not match any variant"),
            "the untagged fallback message is what this impl exists to avoid: {err}"
        );
    }

    #[test]
    fn a_bad_image_object_does_not_report_as_a_string_image() {
        // The failure mode of dispatching by trial: a non-string value must be judged
        // as an object and report the object's error, never "expected a string".
        let err = serde_json::from_str::<Image>(IMAGE_WITH_BAD_BUILD_ARGS)
            .unwrap_err()
            .to_string();
        assert!(!err.contains("expected a string"), "{err}");
    }

    #[test]
    fn both_image_forms_still_deserialize() {
        // Backward compatibility for the hand-written impl: the bare-filename form and
        // the object form must both parse exactly as they did under `untagged`.
        let s: Image = serde_json::from_str(r#""avocado-image-var.btrfs""#).unwrap();
        assert!(matches!(s, Image::String(ref f) if f == "avocado-image-var.btrfs"));

        let o: Image = serde_json::from_str(
            r#"{"out": "boot.img", "size": 64, "size_unit": "mebibytes",
                "block_size": 512, "uuid": "abcd"}"#,
        )
        .unwrap();
        assert_eq!(o.out(), "boot.img");
        assert_eq!(o.size(), Some(64));
        assert_eq!(o.size_unit(), Some("mebibytes"));
        assert_eq!(o.block_size(), Some(512));
        assert_eq!(o.uuid(), Some("abcd"));
        assert!(o.build_args().is_none(), "build_args stays optional");
    }

    #[test]
    fn an_image_object_round_trips_through_serde() {
        // Serialize is still derived while Deserialize is hand-written; this pins the
        // two to the same field names, which a mismatch would otherwise hide until a
        // merged manifest failed to re-read.
        let original: Image = serde_json::from_str(
            r#"{"out": "boot.img", "size": 64, "size_unit": "mebibytes",
                "build_args": {"type": "fat", "variant": "FAT32",
                               "label": "BOOT",
                               "files_append": [{"in": "a.dtbo", "out": "overlays/a.dtbo"}]}}"#,
        )
        .unwrap();
        let reparsed: Image = serde_json::from_str(&serde_json::to_string(&original).unwrap())
            .expect("what Serialize writes, Deserialize must accept");
        assert_eq!(reparsed.all_files().unwrap().len(), 1);
        assert_eq!(reparsed.build_args().unwrap().fat_label(), Some("BOOT"));
    }

    #[test]
    fn a_missing_required_image_field_names_that_field() {
        let err = serde_json::from_str::<Image>(r#"{"out": "boot.img", "size": 64}"#)
            .unwrap_err()
            .to_string();
        assert!(err.contains("size_unit"), "{err}");
        assert!(!err.contains("did not match any variant"), "{err}");
    }

    #[test]
    fn manifest_parse_errors_carry_no_error_prefix() {
        // `main` routes every Err through `log_error`, which adds `[ERROR]`; a literal
        // one here printed it twice. Harmless until deny_unknown_fields made ordinary
        // typos take this path.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.json");
        std::fs::write(&path, "{ not json").unwrap();
        let err = Manifest::from_file(&path).unwrap_err();
        assert!(!err.contains("[ERROR]"), "log_error owns the prefix: {err}");
    }
}
