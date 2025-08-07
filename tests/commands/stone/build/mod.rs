use assert_cmd::Command;
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
