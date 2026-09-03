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
