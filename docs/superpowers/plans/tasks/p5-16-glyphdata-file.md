# P5-16 Glyph data file and local overlays

## Scope

Ship a versioned, readable GlyphData JSON database in `fc-glyphdata`. Resolve
name, Unicode, category, sub-category, script, production name, default anchors,
and composition with existing AGLFN/UCD fallbacks. Apply a validated per-name
project overlay without changing model, document, API, or UI code.

## Owned files and public interfaces

- `crates/fc-glyphdata/Cargo.toml`, `src/lib.rs`, new `src/database.rs` and
  focused database validation/tests, `data/glyphdata.json`, and `data/README.md`.
- `GlyphInfo` contains resolved values; `GlyphInfoOverride` contains optional
  per-field values. `GlyphDataOverlay { version, glyphs }` is serializable.
- `BUNDLED_DATABASE_JSON`, `parse_overlay`, `validate_overlay`, `resolve_info`,
  and inverse scalar lookup `resolve_unicode` are exported from the crate root.
- Unspecified overlay fields inherit; explicit empty arrays clear. Known names
  determine their Unicode ahead of caller fallback values. Suffix variants
  inherit metadata but stay unencoded unless their exact override encodes them.
- JSON version 1; unknown fields remain forward compatible. Reject unsupported
  versions, invalid scalar values, duplicate entries, unknown categories, bad
  names, and excessive document/collection/string lengths.

## Acceptance

Write focused tests first for bundled data, UCD/name fallback, complete/partial
overlays, suffix inheritance, explicit empty overrides, malformed input, and
resource limits. Run `cargo fmt`, scoped clippy and `cargo test -p fc-glyphdata`
only when coordinated with the parent Cargo owner. Parent handles workspace-wide
verification. Run the artifact checker before and after each build batch and
before reporting; use only the shared space-saving Cargo profiles.

No FontForge source or data is adapted. The curated file uses original
FontCarver metadata with existing AGLFN names and Unicode code points.
