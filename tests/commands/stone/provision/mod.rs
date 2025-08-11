use assert_cmd::Command;
use predicates;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_provision_missing_manifest() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    Command::cargo_bin("stone")
        .unwrap()
        .args(["provision", "--input-dir", &input_path.to_string_lossy()])
        .assert()
        .failure()
        .stdout(predicates::str::contains(
            "Manifest file 'manifest.json' not found",
        ));
}

#[test]
fn test_provision_creates_build_dir() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a minimal manifest with no build operations
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch"
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

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();
    fs::write(input_path.join("simple.img"), "test content").unwrap();

    // Create os-release file for AVOCADO_OS_VERSION
    let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="1.0.0"
VERSION_CODENAME=jammy
PRETTY_NAME="Test Linux 1.0.0"
VENDOR_NAME="Test Corporation""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "provision",
            "--input-dir",
            &input_path.to_string_lossy(),
            "--verbose",
        ])
        .assert()
        .success();

    // Check that _build directory was created
    assert!(input_path.join("_build").exists());
}

#[test]
fn test_provision_fat_image_without_fwup() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create test files
    fs::write(input_path.join("test_file.txt"), "Hello, FAT!").unwrap();

    // Create os-release file for AVOCADO_OS_VERSION
    let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="1.0.0"
VERSION_CODENAME=jammy
PRETTY_NAME="Test Linux 1.0.0"
VENDOR_NAME="Test Corporation""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    // Create a manifest with just a FAT image (no fwup required)
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch"
        },
        "storage_devices": {
            "test_device": {
                "out": "test.img",
                "devpath": "/dev/test",
                "images": {
                    "fat_image": {
                        "out": "fat_test.img",
                        "size": 16,
                        "size_unit": "megabytes",
                        "build_args": {
                            "type": "fat",
                            "variant": "FAT32",
                            "files": [
                                "test_file.txt"
                            ]
                        }
                    }
                },
                "partitions": []
            }
        }
    }"#;

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();

    // Create os-release file for AVOCADO_OS_VERSION
    let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="1.0.0"
VERSION_CODENAME=jammy
PRETTY_NAME="Test Linux 1.0.0"
VENDOR_NAME="Test Corporation""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    let result = Command::cargo_bin("stone")
        .unwrap()
        .args([
            "provision",
            "--input-dir",
            &input_path.to_string_lossy(),
            "--verbose",
        ])
        .assert();

    // This should succeed since we're only building FAT images
    result.success();

    // Check that the FAT image was created in _build directory
    assert!(input_path.join("_build").join("fat_test.img").exists());

    // Check that the temporary manifest was cleaned up
    assert!(
        !input_path
            .join("_build")
            .join("temp_manifest_fat_image.json")
            .exists()
    );
}

#[test]
fn test_provision_unsupported_size_unit() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a manifest with unsupported size unit
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch"
        },
        "storage_devices": {
            "test_device": {
                "out": "test.img",
                "devpath": "/dev/test",
                "images": {
                    "bad_image": {
                        "out": "bad.img",
                        "size": 100,
                        "size_unit": "parsecs",
                        "build_args": {
                            "type": "fat",
                            "variant": "FAT32",
                            "files": []
                        }
                    }
                },
                "partitions": []
            }
        }
    }"#;

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();

    // Create os-release file for AVOCADO_OS_VERSION
    let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="1.0.0"
VERSION_CODENAME=jammy
PRETTY_NAME="Test Linux 1.0.0"
VENDOR_NAME="Test Corporation""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    Command::cargo_bin("stone")
        .unwrap()
        .args(["provision", "--input-dir", &input_path.to_string_lossy()])
        .assert()
        .failure()
        .stdout(predicates::str::contains("Unsupported size unit: parsecs"));
}

#[test]
fn test_provision_missing_os_release() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a manifest with fwup build args (which will try to read os-release)
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch"
        },
        "storage_devices": {
            "test_device": {
                "out": "test.fw",
                "devpath": "/dev/test",
                "build_args": {
                    "type": "fwup",
                    "template": "test.conf"
                },
                "images": {},
                "partitions": []
            }
        }
    }"#;

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();
    fs::write(input_path.join("test.conf"), "# Dummy fwup config").unwrap();

    // Don't create os-release file to test the error case
    Command::cargo_bin("stone")
        .unwrap()
        .args(["provision", "--input-dir", &input_path.to_string_lossy()])
        .assert()
        .failure()
        .stdout(predicates::str::contains(
            "OS release file 'os-release' not found",
        ));
}

#[test]
fn test_provision_invalid_os_release() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a manifest with fwup build args (which will try to read os-release)
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch"
        },
        "storage_devices": {
            "test_device": {
                "out": "test.fw",
                "devpath": "/dev/test",
                "build_args": {
                    "type": "fwup",
                    "template": "test.conf"
                },
                "images": {},
                "partitions": []
            }
        }
    }"#;

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();
    fs::write(input_path.join("test.conf"), "# Dummy fwup config").unwrap();

    // Create os-release file without VERSION_ID
    let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
PRETTY_NAME="Test Linux 1.0.0""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    Command::cargo_bin("stone")
        .unwrap()
        .args(["provision", "--input-dir", &input_path.to_string_lossy()])
        .assert()
        .failure()
        .stdout(predicates::str::contains(
            "VERSION_ID field not found in os-release file",
        ));
}

#[test]
fn test_provision_with_provision_script() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a minimal manifest with provision field
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

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();
    fs::write(input_path.join("simple.img"), "test content").unwrap();

    // Create provision script that creates a marker file
    let provision_script = r#"#!/bin/bash
echo "Provision script executed" > provision_output.txt
exit 0
"#;
    fs::write(input_path.join("provision.sh"), provision_script).unwrap();

    // Make the script executable (on Unix systems)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(input_path.join("provision.sh"))
            .unwrap()
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(input_path.join("provision.sh"), perms).unwrap();
    }

    // Create os-release file for AVOCADO_OS_VERSION
    let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="1.0.0"
VERSION_CODENAME=jammy
PRETTY_NAME="Test Linux 1.0.0"
VENDOR_NAME="Test Corporation""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "provision",
            "--input-dir",
            &input_path.to_string_lossy(),
            "--verbose",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Executing provision script"))
        .stdout(predicates::str::contains(
            "Provision script 'provision.sh' completed successfully",
        ));

    // Check that _build directory was created
    assert!(input_path.join("_build").exists());

    // Check that provision script was executed (marker file created)
    assert!(input_path.join("provision_output.txt").exists());
    let output_content = fs::read_to_string(input_path.join("provision_output.txt")).unwrap();
    assert!(output_content.contains("Provision script executed"));
}

#[test]
fn test_provision_with_failing_provision_script() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a minimal manifest with provision field
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

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();
    fs::write(input_path.join("simple.img"), "test content").unwrap();

    // Create provision script that fails
    let provision_script = r#"#!/bin/bash
echo "Provision script failed" >&2
exit 1
"#;
    fs::write(input_path.join("provision.sh"), provision_script).unwrap();

    // Make the script executable (on Unix systems)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(input_path.join("provision.sh"))
            .unwrap()
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(input_path.join("provision.sh"), perms).unwrap();
    }

    // Create os-release file for AVOCADO_OS_VERSION
    let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="1.0.0"
VERSION_CODENAME=jammy
PRETTY_NAME="Test Linux 1.0.0"
VENDOR_NAME="Test Corporation""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    Command::cargo_bin("stone")
        .unwrap()
        .args(["provision", "--input-dir", &input_path.to_string_lossy()])
        .assert()
        .failure()
        .stdout(predicates::str::contains(
            "Provision script 'provision.sh' failed",
        ));
}

#[test]
fn test_provision_builds_images_before_storage_device() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create test files
    fs::write(input_path.join("boot_file.txt"), "Boot content").unwrap();
    fs::write(input_path.join("fwup_template.conf"), "# Dummy fwup template").unwrap();

    // Create os-release file
    let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="1.0.0"
VERSION_CODENAME=jammy
PRETTY_NAME="Test Linux 1.0.0"
VENDOR_NAME="Test Corporation""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    // Create a manifest that has both FAT images and fwup storage device
    // The fwup template would reference the FAT image output file
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch"
        },
        "storage_devices": {
            "rootdisk": {
                "out": "rootdisk.fw",
                "devpath": "/dev/test",
                "build_args": {
                    "type": "fwup",
                    "template": "fwup_template.conf"
                },
                "images": {
                    "boot": {
                        "out": "boot.img",
                        "size": 16,
                        "size_unit": "megabytes",
                        "build_args": {
                            "type": "fat",
                            "variant": "FAT32",
                            "files": [
                                "boot_file.txt"
                            ]
                        }
                    },
                    "simple_image": "simple.img"
                },
                "partitions": []
            }
        }
    }"#;

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();
    fs::write(input_path.join("simple.img"), "Simple image content").unwrap();

    let result = Command::cargo_bin("stone")
        .unwrap()
        .args([
            "provision",
            "--input-dir",
            &input_path.to_string_lossy(),
            "--verbose",
        ])
        .assert();

    // The provision should succeed - meaning FAT images were built first,
    // then fwup was attempted (even if it fails due to missing fwup binary)
    // We verify the build order by checking that:
    // 1. FAT image was created in _build
    // 2. The output shows images being built before storage device
    let output = result.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that boot.img (FAT image) was built
    assert!(
        input_path.join("_build").join("boot.img").exists(),
        "FAT image should be built before fwup attempts to use it"
    );

    // Check build order in log output
    let building_image_pos = stdout.find("Building image 'boot'");
    let building_storage_pos = stdout.find("Building storage device 'rootdisk'");

    // Both should be present, and image should come before storage device
    assert!(building_image_pos.is_some(), "Should log building image");
    assert!(building_storage_pos.is_some(), "Should log building storage device");
    assert!(
        building_image_pos.unwrap() < building_storage_pos.unwrap(),
        "Images should be built before storage device. Image build at {}, Storage build at {}",
        building_image_pos.unwrap(),
        building_storage_pos.unwrap()
    );
}

#[test]
fn test_provision_env_vars_use_full_paths() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create test files
    fs::write(input_path.join("input_file.txt"), "Input content").unwrap();
    fs::write(input_path.join("simple.img"), "Simple image content").unwrap();

    // Create os-release file
    let os_release_content = r#"NAME="Test Linux"
VERSION="1.0.0"
ID=testlinux
VERSION_ID="1.0.0"
VERSION_CODENAME=jammy
PRETTY_NAME="Test Linux 1.0.0"
VENDOR_NAME="Test Corporation""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    // Create a manifest with mixed image types to test path resolution
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch"
        },
        "storage_devices": {
            "test_device": {
                "out": "test.fw",
                "devpath": "/dev/test",
                "build_args": {
                    "type": "fwup",
                    "template": "test_template.conf"
                },
                "images": {
                    "string_image": "simple.img",
                    "generated_image": {
                        "out": "generated.img",
                        "size": 16,
                        "size_unit": "megabytes",
                        "build_args": {
                            "type": "fat",
                            "variant": "FAT32",
                            "files": [
                                "input_file.txt"
                            ]
                        }
                    },
                    "object_no_build": {
                        "out": "object_input.img",
                        "size": 32,
                        "size_unit": "megabytes"
                    }
                },
                "partitions": []
            }
        }
    }"#;

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();
    fs::write(input_path.join("object_input.img"), "Object input content").unwrap();

    // Create a minimal fwup template that will at least parse
    let fwup_template = r#"
# Minimal fwup template for testing
meta-product = "Test Product"
meta-description = "Test Description"
meta-version = "1.0.0"

# Define resources that reference the environment variables
file-resource boot {
    host-path = "${AVOCADO_IMAGE_GENERATED_IMAGE}"
}

file-resource simple {
    host-path = "${AVOCADO_IMAGE_STRING_IMAGE}"
}

file-resource object {
    host-path = "${AVOCADO_IMAGE_OBJECT_NO_BUILD}"
}

task complete {
    # Empty task for testing
}
"#;
    fs::write(input_path.join("test_template.conf"), fwup_template).unwrap();

    let result = Command::cargo_bin("stone")
        .unwrap()
        .args([
            "provision",
            "--input-dir",
            &input_path.to_string_lossy(),
            "--verbose",
        ])
        .assert();

    let output = result.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check that the generated FAT image was created
    assert!(
        input_path.join("_build").join("generated.img").exists(),
        "Generated FAT image should be created"
    );

    // Look for environment variables in the debug output
    // The paths should be absolute, not relative filenames

    // Check that AVOCADO_IMAGE_STRING_IMAGE points to input directory
    let string_image_env = format!("AVOCADO_IMAGE_STRING_IMAGE={}", input_path.join("simple.img").display());
    assert!(
        stdout.contains(&string_image_env),
        "String image should point to input directory. Expected '{}' in output",
        string_image_env
    );

    // Check that AVOCADO_IMAGE_GENERATED_IMAGE points to build directory
    let generated_image_env = format!("AVOCADO_IMAGE_GENERATED_IMAGE={}", input_path.join("_build").join("generated.img").display());
    assert!(
        stdout.contains(&generated_image_env),
        "Generated image should point to build directory. Expected '{}' in output",
        generated_image_env
    );

    // Check that AVOCADO_IMAGE_OBJECT_NO_BUILD points to input directory (no build_args)
    let object_image_env = format!("AVOCADO_IMAGE_OBJECT_NO_BUILD={}", input_path.join("object_input.img").display());
    assert!(
        stdout.contains(&object_image_env),
        "Object image without build_args should point to input directory. Expected '{}' in output",
        object_image_env
    );

    // Verify that AVOCADO_SDK_RUNTIME_DIR is no longer set
    assert!(
        !stdout.contains("AVOCADO_SDK_RUNTIME_DIR="),
        "AVOCADO_SDK_RUNTIME_DIR should no longer be set"
    );
}
