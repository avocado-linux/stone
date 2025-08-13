use assert_cmd::Command;
use predicates;
use std::fs;
use std::os::unix::fs::PermissionsExt;
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
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
VENDOR_NAME="Avocado Linux""#;
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
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
VENDOR_NAME="Avocado Linux""#;
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
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
VENDOR_NAME="Avocado Linux""#;
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
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
VENDOR_NAME="Avocado Linux""#;
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
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
PRETTY_NAME="Avocado Linux 1.0.0""#;
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
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
VENDOR_NAME="Avocado Linux""#;
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
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
VENDOR_NAME="Avocado Linux""#;
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
    fs::write(
        input_path.join("fwup_template.conf"),
        "# Dummy fwup template",
    )
    .unwrap();

    // Create os-release file
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
VENDOR_NAME="Avocado Linux""#;
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
    assert!(
        building_storage_pos.is_some(),
        "Should log building storage device"
    );
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
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
VENDOR_NAME="Avocado Linux""#;
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
meta-product = "Avocado Test Image"
meta-description = "Generic test image for Avocado"
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
    let string_image_env = format!(
        "AVOCADO_IMAGE_STRING_IMAGE={}",
        input_path.join("simple.img").display()
    );
    assert!(
        stdout.contains(&string_image_env),
        "String image should point to input directory. Expected '{string_image_env}' in output"
    );

    // Check that AVOCADO_IMAGE_GENERATED_IMAGE points to build directory
    let generated_image_env = format!(
        "AVOCADO_IMAGE_GENERATED_IMAGE={}",
        input_path.join("_build").join("generated.img").display()
    );
    assert!(
        stdout.contains(&generated_image_env),
        "Generated image should point to build directory. Expected '{generated_image_env}' in output"
    );

    // Check that AVOCADO_IMAGE_OBJECT_NO_BUILD points to input directory (no build_args)
    let object_image_env = format!(
        "AVOCADO_IMAGE_OBJECT_NO_BUILD={}",
        input_path.join("object_input.img").display()
    );
    assert!(
        stdout.contains(&object_image_env),
        "Object image without build_args should point to input directory. Expected '{object_image_env}' in output"
    );

    // Verify that AVOCADO_SDK_RUNTIME_DIR is no longer set
    assert!(
        !stdout.contains("AVOCADO_SDK_RUNTIME_DIR="),
        "AVOCADO_SDK_RUNTIME_DIR should no longer be set"
    );
}

#[test]
fn test_provision_fwup_image_with_disk_env_vars() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create os-release file
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
VENDOR_NAME="Avocado Linux""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    // Create a manifest with a fwup image that has block_size and uuid
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch"
        },
        "storage_devices": {
            "test_device": {
                "out": "test.fw",
                "devpath": "/dev/test",
                "images": {
                    "fwup_image": {
                        "out": "custom.fw",
                        "size": 128,
                        "size_unit": "megabytes",
                        "block_size": 4096,
                        "uuid": "12345678-1234-1234-1234-123456789abc",
                        "build_args": {
                            "type": "fwup",
                            "template": "fwup_template.conf"
                        }
                    }
                },
                "partitions": []
            }
        }
    }"#;

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();

    // Create a minimal fwup template
    let fwup_template = r#"
# Minimal fwup template for testing
meta-product = "Avocado Test Image"
meta-description = "Generic test image for Avocado"
meta-version = "1.0.0"

# Define a resource that would use the disk environment variables
file-resource disk-image {
    # In real usage, this might reference the UUID or block size
    host-path = "/dev/null"
}

task complete {
    # Empty task for testing
}
"#;
    fs::write(input_path.join("fwup_template.conf"), fwup_template).unwrap();

    // Run provision command - it will fail due to missing fwup but we can check the log output
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
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check that the disk-specific environment variables are logged
    // The verbose output should show these environment variables when building the fwup image
    assert!(
        stdout.contains("AVOCADO_DISK_BLOCK_SIZE=4096")
            || stderr.contains("AVOCADO_DISK_BLOCK_SIZE=4096"),
        "Should log AVOCADO_DISK_BLOCK_SIZE environment variable. Stdout: {stdout}, Stderr: {stderr}"
    );

    assert!(
        stdout.contains("AVOCADO_DISK_UUID=12345678-1234-1234-1234-123456789abc")
            || stderr.contains("AVOCADO_DISK_UUID=12345678-1234-1234-1234-123456789abc"),
        "Should log AVOCADO_DISK_UUID environment variable. Stdout: {stdout}, Stderr: {stderr}"
    );

    // Check that we're building the correct fwup image
    assert!(
        stdout.contains("Building fwup image 'fwup_image'")
            || stderr.contains("Building fwup image 'fwup_image'"),
        "Should be building the fwup image"
    );
}

#[test]
fn test_provision_fwup_image_without_disk_env_vars() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create os-release file
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
VENDOR_NAME="Avocado Linux""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    // Create a manifest with a fwup image that does NOT have block_size and uuid
    let manifest_content = r#"{
        "runtime": {
            "platform": "test-platform",
            "architecture": "noarch"
        },
        "storage_devices": {
            "test_device": {
                "out": "test.fw",
                "devpath": "/dev/test",
                "images": {
                    "fwup_image": {
                        "out": "custom.fw",
                        "size": 128,
                        "size_unit": "megabytes",
                        "build_args": {
                            "type": "fwup",
                            "template": "fwup_template.conf"
                        }
                    }
                },
                "partitions": []
            }
        }
    }"#;

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();

    // Create a minimal fwup template
    let fwup_template = r#"
# Minimal fwup template for testing
meta-product = "Avocado Test Image"
meta-description = "Generic test image for Avocado"
meta-version = "1.0.0"

task complete {
    # Empty task for testing
}
"#;
    fs::write(input_path.join("fwup_template.conf"), fwup_template).unwrap();

    // Run provision command
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
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check that the disk-specific environment variables are NOT logged when not present
    assert!(
        !stdout.contains("AVOCADO_DISK_BLOCK_SIZE=")
            && !stderr.contains("AVOCADO_DISK_BLOCK_SIZE="),
        "Should not log AVOCADO_DISK_BLOCK_SIZE when not present in manifest"
    );

    assert!(
        !stdout.contains("AVOCADO_DISK_UUID=") && !stderr.contains("AVOCADO_DISK_UUID="),
        "Should not log AVOCADO_DISK_UUID when not present in manifest"
    );

    // Check that we're still building the fwup image
    assert!(
        stdout.contains("Building fwup image 'fwup_image'")
            || stderr.contains("Building fwup image 'fwup_image'"),
        "Should be building the fwup image"
    );
}

#[test]
fn test_provision_storage_device_with_disk_env_vars() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create test files
    fs::write(input_path.join("boot_file.txt"), "Boot content").unwrap();

    // Create os-release file
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
VENDOR_NAME="Avocado Linux""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    // Create a manifest with storage device that has block_size and uuid (like your imx93 example)
    let manifest_content = r#"{
        "runtime": {
            "platform": "generic-platform",
            "architecture": "test-arch"
        },
        "storage_devices": {
            "rootdisk": {
                "out": "test-rootdisk.zip",
                "build_args": {
                    "type": "fwup",
                    "template": "rootdisk.conf"
                },
                "devpath": "/dev/generic",
                "block_size": 512,
                "uuid": "4bc367b3-5d70-4289-b24d-9b09cb79685c",
                "images": {
                    "boot": {
                        "out": "boot.img",
                        "size": 128,
                        "size_unit": "mebibytes",
                        "build_args": {
                            "type": "fat",
                            "variant": "FAT32",
                            "files": [
                                "boot_file.txt"
                            ]
                        }
                    }
                },
                "partitions": []
            }
        }
    }"#;

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();

    // Create a minimal fwup template
    let fwup_template = r#"
# Minimal fwup template for testing storage device
meta-product = "Avocado Test Image"
meta-description = "Generic test image for Avocado"
meta-version = "1.0.0"

# Example of how fwup template would use the disk environment variables
%if defined(AVOCADO_DISK_UUID)
    meta-uuid = "${AVOCADO_DISK_UUID}"
%endif

task complete {
    # Empty task for testing
}
"#;
    fs::write(input_path.join("rootdisk.conf"), fwup_template).unwrap();

    // Run provision command
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
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check that the storage device disk-specific environment variables are logged
    assert!(
        stdout.contains("AVOCADO_DISK_BLOCK_SIZE=512")
            || stderr.contains("AVOCADO_DISK_BLOCK_SIZE=512"),
        "Should log AVOCADO_DISK_BLOCK_SIZE from storage device. Stdout: {stdout}, Stderr: {stderr}"
    );

    assert!(
        stdout.contains("AVOCADO_DISK_UUID=4bc367b3-5d70-4289-b24d-9b09cb79685c")
            || stderr.contains("AVOCADO_DISK_UUID=4bc367b3-5d70-4289-b24d-9b09cb79685c"),
        "Should log AVOCADO_DISK_UUID from storage device. Stdout: {stdout}, Stderr: {stderr}"
    );

    // Check that we're building the storage device
    assert!(
        stdout.contains("Building storage device 'rootdisk'")
            || stderr.contains("Building storage device 'rootdisk'"),
        "Should be building the rootdisk storage device"
    );

    // Check that the boot.img FAT image was built first
    assert!(
        input_path.join("_build").join("boot.img").exists(),
        "FAT image should be built before storage device"
    );
}

#[test]
fn test_provision_with_profile_system() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path();

    // Create a manifest with provision profiles
    let manifest_content = r#"{
        "runtime": {
            "platform": "avocado-raspberrypi4",
            "architecture": "arm64",
            "provision_default": "img"
        },
        "provision": {
            "envs": {
                "device_info": {
                    "AVOCADO_DEVICE_CERT": "device-cert-value",
                    "AVOCADO_DEVICE_KEY": "device-key-value",
                    "AVOCADO_DEVICE_ID": "device-123"
                }
            },
            "profiles": {
                "img": {
                    "script": "stone-provision-img.sh",
                    "envs": ["device_info"]
                },
                "sd": {
                    "script": "stone-provision-sd.sh",
                    "envs": [
                        "device_info",
                        {"SD_SPECIFIC": "sd-value"}
                    ]
                }
            }
        },
        "storage_devices": {
            "test_device": {
                "out": "test.img",
                "devpath": "/dev/test",
                "images": {},
                "partitions": []
            }
        }
    }"#;

    fs::write(input_path.join("manifest.json"), manifest_content).unwrap();

    // Create os-release file
    let os_release_content = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
VENDOR_NAME="Avocado Linux""#;
    fs::write(input_path.join("os-release"), os_release_content).unwrap();

    // Create provision scripts that output environment variables
    let img_script_content = r#"#!/bin/bash
echo "IMG Script executed" > provision_img_output.txt
echo "AVOCADO_DEVICE_CERT=$AVOCADO_DEVICE_CERT" >> provision_img_output.txt
echo "AVOCADO_DEVICE_KEY=$AVOCADO_DEVICE_KEY" >> provision_img_output.txt
echo "AVOCADO_DEVICE_ID=$AVOCADO_DEVICE_ID" >> provision_img_output.txt
"#;
    fs::write(
        input_path.join("stone-provision-img.sh"),
        img_script_content,
    )
    .unwrap();
    fs::set_permissions(
        input_path.join("stone-provision-img.sh"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    let sd_script_content = r#"#!/bin/bash
echo "SD Script executed" > provision_sd_output.txt
echo "AVOCADO_DEVICE_CERT=$AVOCADO_DEVICE_CERT" >> provision_sd_output.txt
echo "SD_SPECIFIC=$SD_SPECIFIC" >> provision_sd_output.txt
"#;
    fs::write(input_path.join("stone-provision-sd.sh"), sd_script_content).unwrap();
    fs::set_permissions(
        input_path.join("stone-provision-sd.sh"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    // Test with default profile (img)
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
        .stdout(predicates::str::contains("Using provision profile 'img'"))
        .stdout(predicates::str::contains(
            "Resolved 3 environment variables",
        ));

    // Check that the img script was executed
    assert!(input_path.join("provision_img_output.txt").exists());
    let img_output = fs::read_to_string(input_path.join("provision_img_output.txt")).unwrap();
    assert!(img_output.contains("IMG Script executed"));
    assert!(img_output.contains("AVOCADO_DEVICE_CERT=device-cert-value"));
    assert!(img_output.contains("AVOCADO_DEVICE_KEY=device-key-value"));
    assert!(img_output.contains("AVOCADO_DEVICE_ID=device-123"));

    // Clean up for next test
    fs::remove_file(input_path.join("provision_img_output.txt")).unwrap();

    // Test with explicit profile (sd)
    Command::cargo_bin("stone")
        .unwrap()
        .env("AVOCADO_PROVISION_PROFILE", "sd")
        .args([
            "provision",
            "--input-dir",
            &input_path.to_string_lossy(),
            "--verbose",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Using provision profile 'sd'"))
        .stdout(predicates::str::contains(
            "Resolved 4 environment variables",
        ));

    // Check that the sd script was executed
    assert!(input_path.join("provision_sd_output.txt").exists());
    let sd_output = fs::read_to_string(input_path.join("provision_sd_output.txt")).unwrap();
    assert!(sd_output.contains("SD Script executed"));
    assert!(sd_output.contains("AVOCADO_DEVICE_CERT=device-cert-value"));
    assert!(sd_output.contains("SD_SPECIFIC=sd-value"));
}
