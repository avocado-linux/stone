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
