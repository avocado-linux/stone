use assert_cmd::Command;

#[test]
fn test_validate() {
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
