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
    assert!(output_path.join("stone.json").exists());

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
        1,
        "Output directory should contain only the manifest file when no images are defined"
    );

    // Verify the manifest file was copied
    assert!(output_path.join("stone.json").exists());
}
