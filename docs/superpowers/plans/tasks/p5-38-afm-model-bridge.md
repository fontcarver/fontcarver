# P5-38 AFM model bridge

## Scope and ownership

Own `crates/fc-formats/src/metrics/bridge.rs` and
`crates/fc-formats/tests/afm_bridge.rs`. Parent owns the metrics parser, data
types, module wiring and final workspace checks. No new dependencies and no
FontForge source or executable use.

## Public interfaces

- `read_afm_path(&Path) -> Result<Font, Error>` imports a bounded AFM file into an
  outline-free 1000-UPM font and retains its complete parsed metrics in user data.
- `write_afm_font(&Font, &MasterId, &Path) -> Result<(), Error>` exports the
  selected master's explicit horizontal metrics, validating before atomic write.
- `apply_to_font(&MetricsFont, &mut Font, &MasterId) -> Result<(), Error>` applies
  explicitly listed widths and horizontal pairs atomically, converting AFM's
  1000-unit metrics into target font units. Other metrics and metadata stay intact.
- `from_font(&Font, &MasterId) -> Result<MetricsFont, Error>` and
  `into_font(MetricsFont) -> Result<Font, Error>` provide the same validated model
  conversion without filesystem access.

## Acceptance

Write tests before implementation. Cover outline-free import, name-derived
Unicode rather than encoding-code guesses, original metric preservation,
semantic AFM roundtrip, non-1000 UPM conversion, computed component bounds,
partial apply preserving unrelated data, and atomic rejection of missing
masters/glyphs/layers, nonfinite/scaled-overflow numbers and unsupported export
metrics. Validate malformed source metadata rather than silently discarding it.
Reject unsupported active feature/layout metrics, class or vertical kerning,
and live geometry effects. A path failure must not replace existing output.

Run focused tests with workspace Cargo target/profiles, serialized by
`.scratch/p5-resume/cargo.lock`, and task TMPDIR under `.scratch/p5-afm-bridge/`.
Check artifacts at start, around build batches and before completion; format
only owned Rust files. Parent runs final broad Clippy/regressions after merging.

## Acceptance report — 2026-09-30

Implemented all five bridge APIs with source metadata retention, name-derived
Unicode, scaled partial application, resolved outline/component bounds and
atomic path writes. Added nine integration tests before implementation for
roundtrip, scaling, target validation, failure atomicity and export limitations.
No dependencies added; no FontForge source read or executable used.

`rustfmt --edition 2021 --check crates/fc-formats/src/metrics/bridge.rs
crates/fc-formats/tests/afm_bridge.rs` passed with exit 0 and no output.

The existing serialized `cargo test -p fc-formats --test afm_bridge` command
reached compilation and exited 101 before executing any tests:

```text
error[E0658]: use of unstable library feature `str_as_str`
   --> crates/fc-formats/src/metrics/afm.rs:326:25
error[E0432]: unresolved import `write_fonts::Validate`
  --> crates/fc-compile/src/layout_compile.rs:21:42
error: could not compile `fc-compile` (lib) due to 1 previous error
error: could not compile `fc-formats` (lib) due to 5 previous errors
```

The same `GlyphName.as_str()` error occurred at AFM lines 340, 341, 375 and 376.
Both blocking owners were notified. No bridge compiler error was emitted, but
the nine tests are **not passed**. Root requested no more Cargo commands and
will run centralized acceptance/Clippy; no owned build process remains.

Final pre-cleanup artifact check passed: build 6.10 GiB / 13,178 files,
temporary 0.21 GiB / 6,358 files, 668.76 GiB free. Removed only this task's
3,035-byte superseded log and empty TMPDIR. No open design questions; final
test execution remains the integration owner's responsibility.
