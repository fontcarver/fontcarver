# P5 vector interchange acceptance

Implemented `fc_formats::vector::{read_svg, write_svg, read_eps, write_eps}`.
SVG source import/export carries horizontal static outlines, family, em size,
basic ascent/descent/cap/x-height metrics, glyph names and scalar Unicode
assignments; aliases remain native repeated glyph definitions. EPS operates on
one explicitly selected glyph with literal filled paths and optional DSC
metadata. SVG source and the sfnt SVG color table remain separate.

Both codecs bound input/output to 32 MiB, enforce finite coordinates and path
counts, reject unsupported drawing/executable semantics, and serialize fully
before atomically replacing a destination. XML DTD/entity resolution is disabled.
SVG accepts M/L/H/V/Q/C/Z and relative/repeated coordinates; elliptical arcs,
smooth shorthand, contextual glyphs, ligatures, kerning, vertical metrics,
transforms and child graphics are deliberately unsupported. EPS accepts one
nonzero fill with multiple contours, never arbitrary PostScript execution.

The scoped SVG integration tests cover semantic roundtrip, source import,
unsupported/hostile syntax, exact path lexical checking, Unicode aliases,
duplicate conflicts, output limits, and keeping an existing destination on
error. An optional independent fontTools/ElementTree decoder inspects emitted
SVG outlines. The EPS acceptance file records its independently delegated tests,
including Ghostscript on only the fixed fixture emitted by our own writer.

Verification at the scope freeze:

- Tests were written before implementation. The first focused Cargo attempt
  failed on missing vector/metrics registration and an in-flight unrelated
  `VariableStroke` exhaustive match; this is not claimed as behavioral evidence.
- Narrow `rustfmt --edition 2021 --config skip_children=true` and
  `git diff --check` passed.
- The queued `cargo test -p fc-formats --test vector` was cancelled before it
  acquired the shared Cargo lock, at root's request to reserve integration
  build capacity. Its exit status was 143: interrupted, **not passed**.
  No additional Cargo jobs were added after root's integration freeze.
- Final `python3 scripts/check_artifacts.py` passed: 6.10 GiB build / 13,178 files,
  0.21 GiB temporary / 6,358 files, 668.76 GiB free after vector scratch cleanup.

Root owns final workspace Clippy/regression checks. The read-only spec reference
was [W3C SVG 1.1 Fonts](https://www.w3.org/TR/SVG11/fonts.html). No FontForge source
was copied or executed. Added direct dependency `roxmltree` 0.21.1 is already
locked transitively and uses MIT OR Apache-2.0; it supplies bounded XML parsing.

Integration commands to run after shared model stabilization:

```sh
cargo test -p fc-formats --test vector --test vector_eps
cargo test -p fc-formats --lib vector::eps::tests
cargo clippy -p fc-formats --all-targets -- -D warnings
```

The task-specific vector scratch directory contained only its empty test temp
directory and was removed after cancelling the queued test. No source, user
fonts, shared dependency stores or another task's data were removed.
