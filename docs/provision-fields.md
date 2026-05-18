# Provisioning field contract

`provision.fields` is an optional, manifest-level map that decorates the
environment variables surfaced through `provision.envs` with type information
and UI hints. It exists so external tooling (notably `avocado-desktop`) can
render an appropriate input control for each variable when prompting the user
to provision a target.

`stone` itself does not consume this map. The values supplied for each variable
still reach profile scripts through stone's existing
`${VAR}` expansion path; `fields` only describes how those variables should be
collected.

## Shape

```jsonc
"provision": {
  "envs": {
    "device_info": {
      "AVOCADO_DEVICE_CERT": "${AVOCADO_DEVICE_CERT}",
      "AVOCADO_DEVICE_KEY":  "${AVOCADO_DEVICE_KEY}",
      "AVOCADO_DEVICE_ID":   "${AVOCADO_DEVICE_ID}"
    },
    "cmdline": {
      "AVOCADO_CMDLINE_EXTRA": "${AVOCADO_CMDLINE_EXTRA}"
    }
  },
  "fields": {
    "AVOCADO_DEVICE_CERT": {
      "type": "file",
      "label": "Device certificate",
      "description": "X.509 device certificate in PEM format."
    },
    "AVOCADO_DEVICE_KEY": {
      "type": "file",
      "label": "Device private key",
      "description": "PEM-encoded private key paired with the device certificate.",
      "secret": true
    },
    "AVOCADO_DEVICE_ID": {
      "type": "string",
      "label": "Device ID",
      "description": "Stable identifier registered with Peridio."
    },
    "AVOCADO_CMDLINE_EXTRA": {
      "type": "multiline",
      "label": "Extra kernel command-line",
      "description": "Appended to the default kernel cmdline.",
      "required": false
    }
  },
  "profiles": { "...": "..." }
}
```

The map is keyed by environment variable name (matching keys in
`provision.envs.<block>`). Each entry describes one user-facing input.

## Field spec

| key           | type                                   | required | default      | notes                                                              |
| ------------- | -------------------------------------- | -------- | ------------ | ------------------------------------------------------------------ |
| `type`        | `"string" \| "multiline" \| "boolean" \| "file"` | yes | —            | The v1 type set. Unknown values should be treated as `"string"`.   |
| `label`       | string                                 | no       | humanized var name | Short label shown beside the input.                          |
| `description` | string                                 | no       | —            | One-line help text shown under the label.                          |
| `required`    | boolean                                | no       | `true`       | When `false`, the user may leave the value empty.                  |
| `default`     | string \| boolean                      | no       | —            | Pre-filled value (string for text/file types, boolean for `boolean`). |
| `secret`      | boolean                                | no       | `false`      | Mask in the UI and avoid logging. Applies to any type.             |

Unknown keys are reserved for future revisions and must be ignored.

## Type semantics

- **`string`** — single-line text input. Submitted to stone as the entered
  string.
- **`multiline`** — multi-line text input. Submitted to stone as the entered
  string, including newlines.
- **`boolean`** — two-state input (checkbox/toggle). Wire format is
  presence-based: `true` is submitted as `"1"` and `false` as the empty string.
  This matches the convention already used by the Tegra `ERASE_*` flags
  (`[ "${ERASE_NVME:-0}" = "1" ] && ...`).
- **`file`** — file picker. The file's contents are submitted as a string
  (matching how `${AVOCADO_DEVICE_CERT}`/`${AVOCADO_DEVICE_KEY}` are consumed
  today, where the env var holds the PEM body rather than a path). Combine with
  `secret: true` for private-key inputs.

## Resolution

For a chosen provisioning profile the UI should:

1. Walk `profile.envs` and collect every variable name referenced — for
   `Named(block)` entries, take the keys of `provision.envs.<block>`; for
   `Inline(map)` entries, take the keys of the map.
2. Skip entries whose value in `provision.envs` is a literal (no `${VAR}`
   substring) — those are not user input.
3. Look up `provision.fields[<var>]` for each remaining name. A missing entry
   means the variable behaves as `{ "type": "string" }`.
4. Render the union of these fields in declaration order (per profile, then per
   block). A variable referenced from multiple blocks is collected once.

The collected values are placed in the caller's process environment under their
variable names; stone's normal `${VAR}` expansion then drops them into the
manifest before invoking the profile script.

## Why a parallel map (and not inline)

`provision.envs.<block>.<VAR>` values are typed as `string` in stone's
`Manifest` schema and must remain so for stone to keep round-tripping manifests
without changes. Putting the type metadata in a sibling map keeps stone
unchanged: `serde` silently drops unknown top-level keys under `provision`, and
`load_and_merge` round-trips the merged JSON through `serde_json::Value` before
deserialising into the typed view, so `fields` survives overlay merging
verbatim.

## Versioning

`provision.fields` is additive in v1. Future type identifiers (e.g. `enum`,
`secret-string`, `serial-device`) will be added with their own keys; consumers
should fall back to `"string"` when they encounter an unfamiliar `type`.
