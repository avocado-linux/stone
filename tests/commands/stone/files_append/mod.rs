//! End-to-end coverage for `files_append`.
//!
//! The unit tests in `src/manifest.rs` exercise `merge_fat_files` in isolation,
//! which is exactly why every consumer outside it was left unwired: nothing
//! drove `bundle`, `create`, `validate` or `describe-manifest` with an append
//! entry, so the accessor could stay `#[allow(dead_code)]` and the suite stayed
//! green. These tests assert observable state after a real command run, per the
//! repo's integration-test rule.

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use std::fs;
use tempfile::TempDir;

const OS_RELEASE: &str = r#"NAME="Avocado Linux"
VERSION="1.0.0"
ID=avocado
VERSION_ID="1.0.0"
VERSION_CODENAME=test
PRETTY_NAME="Avocado Linux 1.0.0"
AVOCADO_OS_BUILD_ID=test-build-id
"#;

/// Inputs for a manifest whose boot image has one base FAT file and one
/// appended one. Both source files exist, so anything that fails to stage the
/// appended one is a wiring bug rather than a missing input.
fn write_inputs(dir: &std::path::Path) {
    fs::write(dir.join("os-release"), OS_RELEASE).unwrap();
    fs::write(dir.join("avocado-image-var.btrfs"), b"dummy var image").unwrap();
    fs::write(dir.join("bzImage"), b"dummy kernel").unwrap();
    fs::write(dir.join("a.dtbo"), b"dummy overlay").unwrap();
}

fn manifest_with_append(files_append: &str) -> String {
    format!(
        r#"{{
        "runtime": {{"platform": "linux", "architecture": "x86_64"}},
        "storage_devices": {{
            "main": {{
                "out": "disk.img",
                "devpath": "/dev/sda",
                "images": {{
                    "var": "avocado-image-var.btrfs",
                    "boot": {{
                        "out": "boot.img",
                        "size": 64,
                        "size_unit": "mebibytes",
                        "build_args": {{
                            "type": "fat",
                            "variant": "FAT32",
                            "files": [{{"in": "bzImage", "out": "bzImage"}}],
                            "files_append": [{files_append}]
                        }}
                    }}
                }},
                "partitions": [
                    {{"name": "boot", "image": "boot", "size": 64, "size_unit": "mebibytes"}},
                    {{"name": "var", "image": "var", "expand": "true",
                     "size": 64, "size_unit": "mebibytes"}}
                ]
            }}
        }}
    }}"#
    )
}

fn run_bundle(
    input: &std::path::Path,
    build_dir: &std::path::Path,
    output: &std::path::Path,
) -> assert_cmd::assert::Assert {
    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "bundle",
            "-m",
            &input.join("manifest.json").to_string_lossy(),
            "--os-release",
            &input.join("os-release").to_string_lossy(),
            "-i",
            &input.to_string_lossy(),
            "-o",
            &output.to_string_lossy(),
            "--build-dir",
            &build_dir.to_string_lossy(),
        ])
        .assert()
}

#[test]
fn bundle_stages_files_append_inputs_beside_the_base_ones() {
    // The blocking one. `stone bundle` succeeds and the bundle records the
    // appended entry, but if only `files` is staged the source never reaches
    // the build dir - and `stone provision` then dies with "not found in any
    // input directory", one command later and far from the cause.
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path();
    write_inputs(input);
    fs::write(
        input.join("manifest.json"),
        manifest_with_append(r#"{"in": "a.dtbo", "out": "overlays/a.dtbo"}"#),
    )
    .unwrap();

    let build_dir = temp_dir.path().join("_build");
    let output = temp_dir.path().join("os-bundle.aos");
    run_bundle(input, &build_dir, &output).success();

    assert!(
        build_dir.join("bzImage").exists(),
        "the base FAT input must be staged"
    );
    assert!(
        build_dir.join("a.dtbo").exists(),
        "the files_append input must be staged too, or provision cannot rebuild the FAT image"
    );
}

#[test]
fn bundle_rejects_an_append_that_collides_with_a_base_output_by_case() {
    // FAT lookup is case-insensitive and `create_file` returns the existing
    // entry without truncating, so a case-variant duplicate overwrites the base
    // boot file in place and leaves its tail readable past the new data. The
    // guard has to catch this before the image is written, not after.
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path();
    write_inputs(input);
    fs::write(
        input.join("manifest.json"),
        manifest_with_append(r#"{"in": "a.dtbo", "out": "BZIMAGE"}"#),
    )
    .unwrap();

    let build_dir = temp_dir.path().join("_build");
    let output = temp_dir.path().join("os-bundle.aos");
    run_bundle(input, &build_dir, &output)
        .failure()
        .stdout(predicates::str::contains("BZIMAGE").or(predicates::str::contains("bzImage")));
}

#[test]
fn a_duplicate_output_within_files_alone_still_builds() {
    // Compat guard. The dedup exists for the append list; a manifest that
    // already had two base `files` entries sharing an output built before this
    // feature and must keep building, or the change is a silent break for
    // manifests nobody touched.
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path();
    write_inputs(input);
    fs::write(input.join("b.bin"), b"second input").unwrap();

    let manifest = r#"{
        "runtime": {"platform": "linux", "architecture": "x86_64"},
        "storage_devices": {
            "main": {
                "out": "disk.img",
                "devpath": "/dev/sda",
                "images": {
                    "var": "avocado-image-var.btrfs",
                    "boot": {
                        "out": "boot.img",
                        "size": 64,
                        "size_unit": "mebibytes",
                        "build_args": {
                            "type": "fat",
                            "variant": "FAT32",
                            "files": [
                                {"in": "bzImage", "out": "shared"},
                                {"in": "b.bin", "out": "shared"}
                            ]
                        }
                    }
                },
                "partitions": [
                    {"name": "boot", "image": "boot", "size": 64, "size_unit": "mebibytes"},
                    {"name": "var", "image": "var", "expand": "true",
                     "size": 64, "size_unit": "mebibytes"}
                ]
            }
        }
    }"#;
    fs::write(input.join("manifest.json"), manifest).unwrap();

    let build_dir = temp_dir.path().join("_build");
    let output = temp_dir.path().join("os-bundle.aos");
    run_bundle(input, &build_dir, &output).success();
}

#[test]
fn describe_manifest_lists_the_appended_file_not_just_its_count() {
    // A misplaced or misspelled `files_append` key is silently ignored, so
    // describe-manifest is the operator's only confirmation the append landed.
    // A count with no name beside it cannot distinguish "landed" from "landed
    // as something else".
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path();
    write_inputs(input);
    fs::write(
        input.join("manifest.json"),
        manifest_with_append(r#"{"in": "a.dtbo", "out": "overlays/a.dtbo"}"#),
    )
    .unwrap();

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "describe-manifest",
            "-m",
            &input.join("manifest.json").to_string_lossy(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("overlays/a.dtbo"));
}

/// A manifest whose FAT build_args carry an unknown key, for the misspelling case.
fn manifest_with_unknown_key(key: &str) -> String {
    manifest_with_append(r#"{"in": "a.dtbo", "out": "overlays/a.dtbo"}"#)
        .replace("\"files_append\"", &format!("\"{key}\""))
}

#[test]
fn bundle_rejects_a_misspelled_files_append() {
    // The silent-ignore case. `file_append` parsed clean, yielded an image with
    // no overlay, and exited 0 - and this key's author is a delivery hook, so
    // nothing reads the output. The board then boots without its device-tree
    // overlay and the only signal is a missing line in describe-manifest.
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path();
    write_inputs(input);
    fs::write(
        input.join("manifest.json"),
        manifest_with_unknown_key("file_append"),
    )
    .unwrap();

    let build_dir = temp_dir.path().join("_build");
    let output = temp_dir.path().join("os-bundle.aos");
    // Refusing is the property under test. The message cannot name the offending
    // key: `Image` is an untagged enum, and serde discards the inner variant
    // errors when every variant fails, so what surfaces is "did not match any
    // variant" with a line number pointing at the end of the enclosing object.
    // Asserting the key name here would pin a diagnostic the parser cannot give.
    run_bundle(input, &build_dir, &output)
        .failure()
        .stdout(predicates::str::contains("Failed to parse manifest JSON"));

    assert!(
        !output.exists(),
        "no bundle may be produced from a manifest whose append key was not understood"
    );
}

#[test]
fn a_valid_manifest_still_parses_after_the_unknown_key_refusal() {
    // The other half: deny_unknown_fields interacts with serde's internal
    // tagging, so the `type` discriminator itself must not read as unknown.
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path();
    write_inputs(input);
    fs::write(
        input.join("manifest.json"),
        manifest_with_append(r#"{"in": "a.dtbo", "out": "overlays/a.dtbo"}"#),
    )
    .unwrap();

    let build_dir = temp_dir.path().join("_build");
    let output = temp_dir.path().join("os-bundle.aos");
    run_bundle(input, &build_dir, &output).success();
}

/// Read a FAT32 image's volume label out of its boot sector. BS_VolLab sits at
/// offset 0x47 for FAT32 and is 11 bytes, space-padded.
fn fat32_volume_label(image: &std::path::Path) -> String {
    let bytes = fs::read(image).unwrap();
    String::from_utf8_lossy(&bytes[0x47..0x47 + 11])
        .trim_end()
        .to_string()
}

fn manifest_with_label(label: &str) -> String {
    manifest_with_append(r#"{"in": "a.dtbo", "out": "overlays/a.dtbo"}"#).replace(
        r#""variant": "FAT32","#,
        &format!(r#""variant": "FAT32", "label": "{label}","#),
    )
}

#[test]
fn a_manifest_label_reaches_the_fat_volume_label() {
    // Three shipped meta-avocado manifests (stm32mp25-dk, orangepi-5-plus,
    // rzv2n-sr-som) set "label": "BOOT" on a fat build_args. src/fat.rs has had
    // the volume-label write all along, but BuildArgs::Fat carried no field to
    // plumb it from, so those boot partitions were labelled FATFS - silently,
    // which is the same class of failure as the ignored files_append key.
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path();
    write_inputs(input);
    fs::write(input.join("manifest.json"), manifest_with_label("BOOT")).unwrap();

    let build_dir = temp_dir.path().join("_build");
    let output = temp_dir.path().join("os-bundle.aos");
    run_bundle(input, &build_dir, &output).success();

    assert_eq!(
        fat32_volume_label(&build_dir.join("boot.img")),
        "BOOT",
        "the manifest's label must reach the image, not be dropped for the FATFS default"
    );
}

#[test]
fn an_absent_label_keeps_the_existing_default() {
    // The other half: adding the field must not change what a manifest without
    // one produces, or every existing image's label moves.
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path();
    write_inputs(input);
    fs::write(
        input.join("manifest.json"),
        manifest_with_append(r#"{"in": "a.dtbo", "out": "overlays/a.dtbo"}"#),
    )
    .unwrap();

    let build_dir = temp_dir.path().join("_build");
    let output = temp_dir.path().join("os-bundle.aos");
    run_bundle(input, &build_dir, &output).success();

    assert_eq!(
        fat32_volume_label(&build_dir.join("boot.img")),
        "FATFS",
        "a manifest with no label must still get the documented default"
    );
}

#[test]
fn describe_manifest_shows_the_label_it_will_write() {
    // describe-manifest is the operator's confirmation surface, and the label was
    // the one build_arg it did not report - so a manifest asking for BOOT looked
    // identical to one asking for nothing.
    let temp_dir = TempDir::new().unwrap();
    let input = temp_dir.path();
    write_inputs(input);
    fs::write(input.join("manifest.json"), manifest_with_label("BOOT")).unwrap();

    Command::cargo_bin("stone")
        .unwrap()
        .args([
            "describe-manifest",
            "-m",
            &input.join("manifest.json").to_string_lossy(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(r#"label: "BOOT""#));
}
