use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn test_create() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path();

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "create",
            "--manifest-path",
            "tests/fixtures/coverage/stone.json",
            "--os-release",
            "tests/fixtures/coverage/os-release",
            "--output-dir",
            &output_path.to_string_lossy(),
            "--input-dir",
            "tests/fixtures/coverage",
        ])
        .assert()
        .success();

    // Check that the images were created
    assert!(output_path.join("image_1").exists());
    assert!(output_path.join("image_2").exists());

    // Check that the fwup template file was copied
    assert!(output_path.join("rootdisk.conf").exists());

    // Check that the manifest file was copied
    assert!(output_path.join("manifest.json").exists());

    // Check that the OS release file was copied
    assert!(output_path.join("os-release").exists());

    // Check that files preserve their input directory structure
    // The file specified as "subdir/file_2" should be copied to "subdir/file_2", not "foo/file_2"
    assert!(output_path.join("subdir/file_2").exists());
    assert!(!output_path.join("foo/file_2").exists());

    // The image files should be copied as-is (not built into FAT)
    // since create command only stages files
    assert!(output_path.join("image_1").exists());
    assert!(output_path.join("image_2").exists());
}

#[test]
fn test_build_partition_without_image() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path();

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "create",
            "--manifest-path",
            "tests/fixtures/partition_without_image/stone.json",
            "--os-release",
            "tests/fixtures/partition_without_image/os-release",
            "--output-dir",
            &output_path.to_string_lossy(),
            "--input-dir",
            "tests/fixtures/partition_without_image",
        ])
        .assert()
        .success();

    // Should not create any image files since there are no images defined
    // But the manifest file should be copied
    let output_dir_entries: Vec<_> = std::fs::read_dir(output_path)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        output_dir_entries.len(),
        2,
        "Output directory should contain only the manifest and os-release files when no images are defined"
    );

    // Verify the manifest file was copied
    assert!(output_path.join("manifest.json").exists());

    // Verify the OS release file was copied
    assert!(output_path.join("os-release").exists());
}

#[test]
fn test_create_with_provision_file() {
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input");
    let output_path = temp_dir.path().join("output");

    fs::create_dir_all(&input_path).unwrap();
    fs::create_dir_all(&output_path).unwrap();

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
    fs::write(input_path.join("os-release"), "NAME=Test\nVERSION_ID=1.0").unwrap();

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "create",
            "--manifest-path",
            &manifest_path.to_string_lossy(),
            "--os-release",
            &input_path.join("os-release").to_string_lossy(),
            "--output-dir",
            &output_path.to_string_lossy(),
            "--input-dir",
            &input_path.to_string_lossy(),
        ])
        .assert()
        .success();

    // Check that the provision file was copied
    assert!(output_path.join("provision.sh").exists());

    // Check that the manifest file was copied
    assert!(output_path.join("manifest.json").exists());

    // Check that the OS release file was copied
    assert!(output_path.join("os-release").exists());

    // Check that the image file was copied
    assert!(output_path.join("simple.img").exists());
}
