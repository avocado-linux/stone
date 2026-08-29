# Changelog

Notable changes to `stone`, newest first.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions
follow [semantic versioning](https://semver.org/spec/v2.0.0.html), where the
public interface is the manifest schema and the CLI, not the Rust API.

Started at 2.3.0. For 2.2.0 and earlier, see the annotated tags and `git log`.

## [Unreleased]

## [2.3.1] - 2026-08-29

### Added

- `provision` emits `AVOCADO_PARTITION_<NAME>_FLAGS` for every partition:
  `0x0100000000000000` when the manifest says `expand: "true"`, `0` otherwise,
  so the fwup templates can write it as the partition's GPT attribute flags. An
  image written to a file and flashed later cannot be expanded at flash time, and
  nothing else on the device knows what the manifest asked for; with the bit on
  the partition the initramfs can grow it to the disk before `/var` is opened.
  Bits 48-63 are the partition type owner's; 56 is Avocado's grow-to-disk mark.
  It squats an unassigned bit in the Discoverable Partitions Specification's
  type-owned range — the templates default `var` to the DPS `/var` GUID and
  stone does not control the type.

### Changed

- **`expand: "true"` must be the last partition and must be named**, in every
  form, not only when `size` is omitted. Now that the flag leaves the build as a
  GPT attribute the device acts on, a sized expand partition followed by another
  would have had the device grow it over its neighbour (fwup only catches that
  when a template wires `expand =` for it), and an unnamed one silently got no
  `AVOCADO_PARTITION_<NAME>_FLAGS` at all. Both are rejected at manifest
  validation. This refuses manifests that parsed under 2.3.0, but such a manifest
  was already not getting what it asked for; the coverage fixture was one.

## [2.3.0] - 2026-08-12

### Added

- `files_append` on a `fat` `build_args`: a second FAT file list, merged onto
  `files` at build time and deduplicated by output path. Lets a delivery hook
  contribute one file to a boot partition without knowing or restating the base
  list. Two overlays that each append a file keep both — `deep_merge_json`
  concatenates this key rather than replacing it.
- `label` on a `fat` `build_args`: sets the FAT volume label. `src/fat.rs` could
  always write it; there was no manifest field to plumb it from.
- `describe-manifest` reports `label` and `files_append`, and lists appended
  entries in the merged `Files` block rather than only counting them.

### Changed

- **Unknown keys inside `build_args` are now refused rather than ignored.** This
  rejects manifests that parsed under 2.2.0 — any carrying a stray or misspelled
  key there. Kept out of a major bump because such keys were never honoured: a
  manifest this breaks was already not getting what it asked for, silently. It is
  also how the missing `label` field was found.
- Parse failures name the offending key and list the valid ones, e.g. ``unknown
  field `file_append`, expected one of `variant`, `files`, `files_append`,
  `label` ``. `Image`'s `Deserialize` is hand-written to make this possible;
  under `#[serde(untagged)]` serde discarded the inner error and reported `data
  did not match any variant of untagged enum Image`, positioned past the actual
  mistake. Applies to any malformed image object — an omitted `size_unit` now
  reports `missing field size_unit`.

### Fixed

- `bundle` staged only base `files`, so a bundle could reference an appended
  source it had not copied and `provision` then failed a command later. All four
  consumers (`bundle`, `create`, `validate`, `describe-manifest`) now read the
  merged list.
- An appended entry colliding with an existing output is a build-time error
  instead of a silent overwrite. Comparison normalizes case and path form,
  because fatfs resolves names case-insensitively and `create_file` returns the
  existing entry *without* truncating — so `overlays/VC4.DTBO` written over
  `overlays/vc4.dtbo` produced one file holding the new bytes followed by the
  tail of the old one. An entry whose `{in,out}` matches one already present
  collapses instead, so a hook re-emitting on every rebuild is a no-op.
- Manifests setting `"label": "BOOT"` were built with the `FATFS` default. Three
  shipped meta-avocado manifests (`stm32mp25-dk`, `orangepi-5-plus`,
  `rzv2n-sr-som`) were affected. Manifests without the key are unchanged.
- Error messages no longer print `[ERROR]` twice. 18 error strings carried a
  literal prefix that doubled against the one `log_error` adds.

### Notes

- Duplicate outputs *within* base `files` are still allowed. Such manifests built
  before `files_append` existed, and the dedup is a property of appending.
- Parse strictness covers `build_args` only. A misspelled key elsewhere is still
  silently ignored — `expnd` for `expand` yields a non-expanding partition with no
  warning.

[Unreleased]: https://github.com/avocado-linux/stone/compare/2.3.0...HEAD
[2.3.0]: https://github.com/avocado-linux/stone/compare/2.2.0...2.3.0
