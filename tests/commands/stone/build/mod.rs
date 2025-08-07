use assert_cmd::Command;
use stone::fat::list_fat_files;
use tempfile::TempDir;

#[test]
fn test_build() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path();

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "build",
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

    // Check that the FAT image contains the expected files
    let fat_image_path = output_path.join("image_1");
    let fat_files = list_fat_files(&fat_image_path).expect("Failed to list FAT files");

    // The manifest specifies these files should be in the FAT image:
    // - "file_1" (direct mapping)
    // - "file_2" -> "foo/file_2" (mapped to subdirectory)
    assert!(
        fat_files.contains(&"file_1".to_string()),
        "FAT image should contain file_1, found files: {fat_files:?}"
    );
    assert!(
        fat_files.contains(&"foo/file_2".to_string()),
        "FAT image should contain foo/file_2, found files: {fat_files:?}"
    );
}

#[test]
fn test_build_partition_without_image() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path();

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "build",
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
    let output_dir_entries: Vec<_> = std::fs::read_dir(output_path)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(
        output_dir_entries.is_empty(),
        "Output directory should be empty when no images are defined"
    );
}
