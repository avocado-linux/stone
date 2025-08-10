use assert_cmd::Command;
use predicates::str::contains;

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
