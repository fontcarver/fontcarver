# P1 Task 3 Report — `fw-compile`

## Status

Complete. Commit `7c4a99d` (`feat(fw-compile): add fontc static export and preservation`) is on `p1-t3`; nothing was pushed.

## Built

- Added the workspace `fw-compile` crate and the normative export API.
- Implemented a single-master in-memory `fontir::Source`: metadata/names, global metrics, outlines/components/anchors, Unicode, vertical metrics, kerning groups/pairs, and in-memory FEA.
- Added fontc TTF compilation, table subsetting/reporting, provenance SHA/order checks, raw/layout-table preservation, DSIG exclusion, and TrueType program/per-glyph instruction reattachment or all-hint dropping with diagnostics.
- Added HarfRust Latin shaping fingerprints, a spec-compliant per-table WOFF1 encoder, and WOFF2 through `woofwoof`. WOFF2's C++11 build requirement is documented.
- Added integration tests and a Criterion DejaVu export benchmark.

## Verification

```text
cargo fmt --all --check
  PASS
cargo clippy -p fw-compile --all-targets -- -D warnings
  PASS
cargo test -p fw-compile
  PASS: 10 integration tests, 0 failed; doc tests pass
cargo test -p fw-compile --release
  PASS: 10 integration tests, 0 failed; doc tests pass
```

Release export timings: DejaVu Sans `509.739623 ms` (budget `<5 s`); Source Han Sans SC `40.977647267 s` (budget `<60 s`). The external Source Han fixture was present; its 65,535-glyph output was parsed and its `maxp` count and U+4E00 cmap mapping checked.

## Concerns / deferred work

Fontc 1.0 overflows its format-4 cmap/post-name paths at the OpenType glyph limit. Large exports therefore suppress those inputs during fontc, emit a format-12 cmap, and replace `post` with format 3, reporting `post-names-omitted-capacity`. Overlap removal and autohint intentionally warn as unavailable. FWF remains owned by `fw-doc`. `icu_properties` remains pinned at `2.1.1`.

## Fix round 1

Status: complete in `7bf2809`, `ffb0ff4`, and `5f32f8a`; nothing was pushed.

- C1: removed per-glyph instruction attachment. The untouched/verified path now copies original `glyf`, `loca`, `maxp`, `fpgm`, `prep`, and `cvt ` bytes and reconciles `head.indexToLocFormat`; every other path uses fontc's instruction-free outlines (`crates/fw-compile/src/preserve.rs:104`, `:326`). `tests/preserve.rs:7` checks byte identity and compares DejaVu `A` points with read-fonts; `:74` checks all instructions/tables disappear after an edit.
- C2: compiled gid order is recovered from post format 2 names, normalized back to model names, and checked against `maxp`; post-less/format-3 output derives fontir order (`.notdef`, then exported source order). The same name map drives the large-font cmap (`crates/fw-compile/src/large_font.rs:61`, `:92`, `:228`). `tests/export_static.rs:67` covers model order `[A, .notdef, B]` and cmap gids.
- C3: deleted the first-glyph rename from `FwSource::from_font`; fontir synthesizes/moves `.notdef` (`crates/fw-compile/src/source.rs:52`). `tests/export_static.rs:88` proves `[A, B]` exports as `[.notdef, A, B]` with U+0041 mapped to `A`. CID import now correctly names gid 0 `.notdef` so 65,535-glyph CFF fonts stay within `maxp` capacity (`crates/fw-sfnt/src/read/names.rs:112`, `tests/read_cjk.rs:14`).
- I1: order changes/unavailable provenance drop all ruled gid-indexed raw tables with one tag-list warning, while variation tables are unconditionally excluded from static output with their own warning (`crates/fw-compile/src/preserve.rs:14`, `:31`, `:136`, `:186`). `tests/preserve.rs:190` injects every ruled tag and verifies absence and both diagnostics.
- I2: all mapped non-Mac languages use `NameKey::new_with_lang`; the BCP-47/LCID table includes the ruled English, European, CJK, Russian, and Brazilian Portuguese mappings. Unknown languages and all Mac records are skipped with the ruled warnings (`crates/fw-compile/src/metadata.rs:17`, `:118`, `:155`). `tests/export_static.rs:105` covers en/de, unknown, and Mac records; `:154` checks DejaVu retains all 13 Windows records.
- I3: serialized `SourceProvenance.glyph_order` is additive/defaulted and populated on sfnt import (`crates/fw-model/src/extra.rs:28`, `crates/fw-sfnt/src/read/mod.rs:132`). Preservation uses that order without source bytes; only the absence of both forms emits `preservation-unavailable-no-provenance` (`crates/fw-compile/src/preserve.rs:60`, `:220`). Model/import serde tests and `tests/preserve.rs:128`, `:157` cover save/reopen and legacy provenance.
- I4: any model UVS data emits `variation-sequences-dropped` (`crates/fw-compile/src/export.rs:128`), covered by `tests/export_static.rs:177`.
- I5: the post decision now uses the exact `258 + unique_nonstandard_names <= 65_535` constraint; fallback retains complete model codepoints in format 12 plus a bounded BMP format 4, and all gid lookup is name-based (`crates/fw-compile/src/large_font.rs:1`, `:22`, `:216`, `:228`). Unit tests at `:395` build 65,400-name fit/overflow cases and at `:415` parse both cmap formats without external fonts. Release Source Han export maps U+4E00 and has 65,535 glyphs.
- Minors: model `head.created` now sets reproducible created/modified timestamps (`metadata.rs:246`, `:285`); subset core includes `fpgm/prep/cvt ` (`export.rs:196`); unhinted edits do not warn (`preserve.rs:104`, `tests/preserve.rs:243`); per-table `value-clamped` counts cover outlines and advances (`export.rs:214`); disabling raw preservation lists dropped tags (`preserve.rs:201`); imported `gasp` bytes retain the model's version (`fw-sfnt/src/read/raw.rs:15`, `preserve.rs:162`); feature class bodies/brackets are retained verbatim with the format documented (`source.rs:491`, `:527`). All have focused regression tests.
- fw-sfnt cmap: modern Unicode/Windows subtables suppress Macintosh (1,0), while Mac-only fonts decode bytes through the full Mac Roman table (`crates/fw-sfnt/src/read/cmap.rs:30`, `:77`, `:92`, `:104`). `tests/read_ttf.rs:98` proves DejaVu no longer gains U+0000/U+0008/U+0080.

Verification final lines:

```text
cargo fmt --all --check
  PASS (exit 0; no output)
cargo clippy -p fw-compile -p fw-sfnt -p fw-model --all-targets -- -D warnings
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
cargo test -p fw-compile -p fw-sfnt -p fw-model
  test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  PASS: 84 tests across the three crates, plus doc-tests
cargo test -p fw-compile --release
  test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  PASS: 27 tests, plus doc-tests
```

Release export timings (`--nocapture`): DejaVu Sans `746.231614 ms`; Source Han Sans SC `54.233375543 s`.
