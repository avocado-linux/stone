use assert_cmd::Command;
use predicates;
use predicates::prelude::PredicateBooleanExt;
use std::fs;
use tempfile::TempDir;

const OS_RELEASE: &str = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
AVOCADO_OS_BUILD_ID=test-build-id
"#;

fn write_minimal_inputs(dir: &std::path::Path) {
    fs::write(dir.join("os-release"), OS_RELEASE).unwrap();
    // A dummy 'var' file referenced by the manifest's images map so artifact
    // collection succeeds. Layout sizing doesn't depend on its byte count.
    fs::write(dir.join("avocado-image-var.btrfs"), b"dummy var image").unwrap();
}

#[test]
fn test_bundle_partition_size_override_applies_alignment() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path();
    write_minimal_inputs(input);

    let manifest = r#"{
        "runtime": {"platform": "linux", "architecture": "x86_64"},
        "storage_devices": {
            "main": {
                "out": "disk.img",
                "devpath": "/dev/sda",
                "images": {
                    "var": "avocado-image-var.btrfs"
                },
                "partitions": [
                    {"name": "boot", "size": 256, "size_unit": "mebibytes"},
                    {"name": "var", "image": "var", "expand": "true",
                     "size_alignment": 16, "size_alignment_unit": "mebibytes"}
                ]
            }
        }
    }"#;
    fs::write(input.join("manifest.json"), manifest).unwrap();

    let build_dir = temp_dir.path().join("_build");
    let output = temp_dir.path().join("os-bundle.aos");

    // Pass 100 MiB raw size. With size_alignment=16 mebibytes this should
    // round up to 112 MiB = 117_440_512 bytes.
    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "bundle",
            "-m",
            &input.join("manifest.json").to_string_lossy(),
            "--os-release",
            &input.join("os-release").to_string_lossy(),
            "-i",
            &input.to_string_lossy(),
            "-o",
            &output.to_string_lossy(),
            "--build-dir",
            &build_dir.to_string_lossy(),
            "--partition-size",
            &format!("var={}", 100 * 1024 * 1024),
        ])
        .assert()
        .success();

    let bundle_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(build_dir.join("bundle.json")).unwrap()).unwrap();

    let parts = bundle_json["layout"]["partitions"].as_array().unwrap();
    assert_eq!(parts.len(), 2);

    // First partition keeps its explicit size, normalized to bytes.
    assert_eq!(parts[0]["name"], "boot");
    assert_eq!(parts[0]["size_unit"], "bytes");
    assert_eq!(parts[0]["size"].as_u64().unwrap(), 256 * 1024 * 1024);

    // Var partition is aligned up to 112 MiB.
    assert_eq!(parts[1]["name"], "var");
    assert_eq!(parts[1]["size_unit"], "bytes");
    assert_eq!(parts[1]["size"].as_u64().unwrap(), 112 * 1024 * 1024);
    assert_eq!(parts[1]["expand"], "true");

    // Offset of var follows boot.
    assert_eq!(parts[1]["offset"].as_u64().unwrap(), 256 * 1024 * 1024);
}

#[test]
fn test_bundle_missing_partition_size_override_errors() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path();
    write_minimal_inputs(input);

    let manifest = r#"{
        "runtime": {"platform": "linux", "architecture": "x86_64"},
        "storage_devices": {
            "main": {
                "out": "disk.img",
                "devpath": "/dev/sda",
                "images": {"var": "avocado-image-var.btrfs"},
                "partitions": [
                    {"name": "boot", "size": 256, "size_unit": "mebibytes"},
                    {"name": "var", "image": "var", "expand": "true"}
                ]
            }
        }
    }"#;
    fs::write(input.join("manifest.json"), manifest).unwrap();

    let build_dir = temp_dir.path().join("_build");
    let output = temp_dir.path().join("os-bundle.aos");

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "bundle",
            "-m",
            &input.join("manifest.json").to_string_lossy(),
            "--os-release",
            &input.join("os-release").to_string_lossy(),
            "-i",
            &input.to_string_lossy(),
            "-o",
            &output.to_string_lossy(),
            "--build-dir",
            &build_dir.to_string_lossy(),
        ])
        .assert()
        .failure()
        .stdout(
            predicates::str::contains("var")
                .and(predicates::str::contains("--partition-size")),
        );
}

#[test]
fn test_bundle_omitted_size_on_non_last_partition_fails_validation() {
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path();
    write_minimal_inputs(input);

    let manifest = r#"{
        "runtime": {"platform": "linux", "architecture": "x86_64"},
        "storage_devices": {
            "main": {
                "out": "disk.img",
                "devpath": "/dev/sda",
                "images": {"var": "avocado-image-var.btrfs"},
                "partitions": [
                    {"name": "var", "image": "var", "expand": "true"},
                    {"name": "boot", "size": 256, "size_unit": "mebibytes"}
                ]
            }
        }
    }"#;
    fs::write(input.join("manifest.json"), manifest).unwrap();

    let build_dir = temp_dir.path().join("_build");
    let output = temp_dir.path().join("os-bundle.aos");

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "bundle",
            "-m",
            &input.join("manifest.json").to_string_lossy(),
            "--os-release",
            &input.join("os-release").to_string_lossy(),
            "-i",
            &input.to_string_lossy(),
            "-o",
            &output.to_string_lossy(),
            "--build-dir",
            &build_dir.to_string_lossy(),
        ])
        .assert()
        .failure()
        .stdout(predicates::str::contains("last partition"));
}
