use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn test_describe_manifest() {
    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "describe-manifest",
            "--manifest-path",
            "tests/fixtures/coverage/stone.json",
        ])
        .assert()
        .success();
}

#[test]
fn test_describe_manifest_with_overlay() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();

    // Overlay changes the platform name
    let overlay_content = r#"{
        "runtime": {
            "platform": "avocado-overlay-test"
        }
    }"#;
    let overlay_path = temp_dir.path().join("overlay.json");
    fs::write(&overlay_path, overlay_content).unwrap();

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "describe-manifest",
            "--manifest-path",
            "tests/fixtures/coverage/stone.json",
            "--overlay",
            &overlay_path.to_string_lossy(),
        ])
        .assert()
        .success()
        .stdout(contains("avocado-overlay-test"));
}
