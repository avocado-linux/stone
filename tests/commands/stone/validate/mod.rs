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

#[test]
fn test_validate_missing_provision_profile_script() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a manifest with provision profiles pointing to non-existent scripts
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch"
        },
        "provision": {
            "profiles": {
                "profile1": {
                    "script": "missing_script.sh"
                },
                "profile2": {
                    "script": "another_missing.sh"
                }
            }
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
        .stdout(contains("missing_script.sh"))
        .stdout(contains("another_missing.sh"));
}

#[test]
fn test_validate_with_provision_profiles() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a manifest with provision profiles
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch"
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
        input_path.join("script1.sh"),
        "#!/bin/bash\necho 'profile1'",
    )
    .unwrap();
    fs::write(
        input_path.join("script2.sh"),
        "#!/bin/bash\necho 'profile2'",
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

#[test]
fn test_validate_missing_provision_default_profile() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a manifest with provision_default referencing non-existent profile
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch",
            "provision_default": "nonexistent_profile"
        },
        "provision": {
            "profiles": {
                "profile1": {
                    "script": "script1.sh"
                }
            }
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
        input_path.join("script1.sh"),
        "#!/bin/bash\necho 'profile1'",
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
        .failure()
        .stdout(contains("Profile 'nonexistent_profile' not found"));
}

#[test]
fn test_validate_provision_default_without_provision_section() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a manifest with provision_default but no provision section
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch",
            "provision_default": "some_profile"
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
        .stdout(contains(
            "provision_default specified but no provision section found",
        ));
}

#[test]
fn test_validate_with_valid_provision_default() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a manifest with valid provision_default
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch",
            "provision_default": "default_profile"
        },
        "provision": {
            "profiles": {
                "default_profile": {
                    "script": "default.sh"
                },
                "other_profile": {
                    "script": "other.sh"
                }
            }
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
        input_path.join("default.sh"),
        "#!/bin/bash\necho 'default provision'",
    )
    .unwrap();
    fs::write(
        input_path.join("other.sh"),
        "#!/bin/bash\necho 'other provision'",
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

#[test]
fn test_validate_missing_provision_default_script() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a manifest with provision_default pointing to profile with missing script
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch",
            "provision_default": "default_profile"
        },
        "provision": {
            "profiles": {
                "default_profile": {
                    "script": "missing_default.sh"
                }
            }
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
        .stdout(contains("missing_default.sh"));
}
