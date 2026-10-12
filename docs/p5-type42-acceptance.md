# P5 Type 42 container acceptance

Implemented the bounded standalone Type 42 codec in
`crates/fc-formats/src/type42/`. Public `read_bytes`, `read_path` and `write_bytes`
use the existing crate Error. No new dependency or FontForge source adaptation.

The writer derives font name, normalized bounding box, 256-code encoding and
CharStrings from TrueType metadata. It retains embedded SFNT bytes and splits
hex strings at table/glyph boundaries with compatibility padding. The reader
recognizes literal arrays and conventional encoding initialization without
executing PostScript, bounds input and decoded sizes, checks SFNT table/glyph
ranges, and rejects executable glyphs, unsupported transforms, metrics overrides,
computed sfnts, duplicate fields and malformed strings.

Reference: [Adobe technical note 5012](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5012.Type42_Spec.pdf).

## Evidence

- `rustfmt --edition 2021 --check crates/fc-formats/src/type42/mod.rs crates/fc-formats/tests/type42.rs`: passed (no output).
- `git diff --check -- crates/fc-formats/src/type42 crates/fc-formats/tests/type42.rs`: passed (no output).
- Eight integration tests added: compiled-font byte/name/encoding round trip,
  independent literal input, real-font multiple strings, invalid SFNT/CFF/TTC,
  malformed/computed arrays, duplicate and semantic overrides, dangerous
  PostScript, and independent Ghostscript metrics/rendering for small and large
  fonts. Ghostscript is installed.
- Focused Cargo test was **interrupted before acquiring the shared lock** when
  root froze agent builds for final integration. Tests, Ghostscript assertions
  and Clippy were **not run locally**; parent final workspace acceptance owns
  execution. No claim of passing runtime acceptance is made here.
- Task-start resource check: build 5.72 GiB; scratch 0.01 GiB; free 669.87 GiB,
  PASS. Completion resource check recorded below.

## Supported boundaries

Input is a hexadecimal declarative Type 42 subset, not a PostScript interpreter.
Binary sfnts and CID wrappers are unsupported. Non-glyf tables or individual
glyphs exceeding 65,534 bytes, and odd-length SFNT buffers, fail explicitly at
export. Import extracts the original SFNT without rewriting cmap/post to match
wrapper aliases or reencoding. Application/API/CLI registration belongs to
remaining-formats/export owners. No commits were made.

Completion resource check: build 6.09 GiB / 13,176 files; temporary 0.22 GiB /
7,911 files; free 668.67 GiB, PASS. Own task scratch removed; no owned Cargo or
interpreter processes remain.
