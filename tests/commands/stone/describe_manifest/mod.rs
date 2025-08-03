use assert_cmd::Command;

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
