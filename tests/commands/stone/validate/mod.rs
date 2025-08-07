use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn test_validate_success() {
    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "validate",
            "--manifest-path",
            "tests/fixtures/coverage/stone.json",
            "--input-dir",
            "tests/fixtures/coverage",
        ])
        .assert()
        .success();
}

#[test]
fn test_validate_partition_without_image_key() {
    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "validate",
            "--manifest-path",
            "tests/fixtures/partition_without_image/stone.json",
            "--input-dir",
            "tests/fixtures/partition_without_image",
        ])
        .assert()
        .success();
}

#[test]
fn test_validate_missing_device_fwup_template() {
    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "validate",
            "--manifest-path",
            "tests/fixtures/missing_device_fwup_template/stone.json",
            "--input-dir",
            "tests/fixtures/missing_device_fwup_template",
        ])
        .assert()
        .failure()
        .stdout(contains("missing_template.conf"));
}

#[test]
fn test_validate_missing_provision_file() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a manifest with provision field pointing to non-existent file
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch",
            "provision": "provision.sh"
        },
        "storage_devices": {
            "test_device": {
                "out": "test.img",
                "devpath": "/dev/test",
                "images": {
                    "simple_image": "simple.img"
                },
                "partitions": []
            }
        }
    }"#;

    let manifest_path = input_path.join("manifest.json");
    fs::write(&manifest_path, manifest_content).unwrap();
    fs::write(input_path.join("simple.img"), "test content").unwrap();

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "validate",
            "--manifest-path",
            &manifest_path.to_string_lossy(),
            "--input-dir",
            &input_path.to_string_lossy(),
        ])
        .assert()
        .failure()
        .stdout(contains("provision.sh"));
}

#[test]
fn test_validate_with_provision_file() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a manifest with provision field
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch",
            "provision": "provision.sh"
        },
        "storage_devices": {
            "test_device": {
                "out": "test.img",
                "devpath": "/dev/test",
                "images": {
                    "simple_image": "simple.img"
                },
                "partitions": []
            }
        }
    }"#;

    let manifest_path = input_path.join("manifest.json");
    fs::write(&manifest_path, manifest_content).unwrap();
    fs::write(input_path.join("simple.img"), "test content").unwrap();
    fs::write(
        input_path.join("provision.sh"),
        "#!/bin/bash\necho 'provisioning'",
    )
    .unwrap();

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "validate",
            "--manifest-path",
            &manifest_path.to_string_lossy(),
            "--input-dir",
            &input_path.to_string_lossy(),
        ])
        .assert()
        .success();
}
