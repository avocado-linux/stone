use assert_cmd::Command;
use tempfile::TempDir;

#[test]
fn test_build() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path();

    Command::cargo_bin("stone")
        .unwrap()
        .args(&[
            "build",
            "--manifest-path",
            "tests/fixtures/coverage/manifest.json",
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
