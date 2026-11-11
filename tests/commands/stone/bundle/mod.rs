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
            predicates::str::contains("var").and(predicates::str::contains("--partition-size")),
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

/// Two bundles of identical inputs must be byte-identical. The device decides
/// whether a release carries an OS update by comparing the sha256 of the whole
/// `.aos`, so any build-time noise (tar header mtimes, uid/gid, artifact order)
/// makes every app-only release write the inactive slot and reboot.
#[test]
fn test_bundle_is_byte_reproducible() {
    let names = ["boot", "kernel", "rootfs", "dtb", "initrd", "firmware"];
    let images: serde_json::Map<String, serde_json::Value> = names
        .iter()
        .map(|n| (n.to_string(), format!("{n}.img").into()))
        .collect();
    let os_artifacts: serde_json::Map<String, serde_json::Value> = names
        .iter()
        .map(|n| {
            let v = serde_json::json!({"image_key": n, "slot_partitions": [format!("{n}_a"), format!("{n}_b")]});
            (n.to_string(), v)
        })
        .collect();
    let manifest = serde_json::json!({
        "runtime": {"platform": "linux", "architecture": "x86_64"},
        "storage_devices": {"main": {"out": "disk.img", "devpath": "/dev/sda", "images": images, "partitions": []}},
        "update": {
            "slot_detection": {"type": "command", "command": ["x"]},
            "os_artifacts": os_artifacts,
            "activate": []
        }
    });

    let bundle = |dir: &std::path::Path| -> Vec<u8> {
        // Fresh inputs every run, so every file gets a new mtime.
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("os-release"), OS_RELEASE).unwrap();
        fs::write(dir.join("manifest.json"), manifest.to_string()).unwrap();
        for n in names {
            fs::write(dir.join(format!("{n}.img")), format!("{n} payload")).unwrap();
        }
        let output = dir.join("os-bundle.aos");
        Command::cargo_bin("stone")
            .unwrap()
            .args([
                "bundle",
                "-m",
                &dir.join("manifest.json").to_string_lossy(),
                "--os-release",
                &dir.join("os-release").to_string_lossy(),
                "-i",
                &dir.to_string_lossy(),
                "-o",
                &output.to_string_lossy(),
            ])
            .assert()
            .success();
        fs::read(output).unwrap()
    };

    let temp_dir = TempDir::new().unwrap();
    let first = bundle(&temp_dir.path().join("one"));
    // Cross a whole second so tar's one-second mtime resolution sees the change.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let second = bundle(&temp_dir.path().join("two"));
    assert!(first == second, "two bundles of the same inputs differ");
}
