# P2d-01 Glyphs interoperability

Implement the Glyphs section of `../../specs/2026-09-19-p2-completion.md`.
Own `crates/fc-formats/src/glyphs/` and `crates/fc-formats/tests/glyphs*.rs`.
Public interfaces: `read_glyphs(&Path) -> Result<Font, crate::Error>` and
`write_glyphs(&Font, &Path) -> Result<(), crate::Error>`. Multi-master source
export writes all masters. Parent owns shared registration and dependencies.
Use `Error::Source { format: "Glyphs", reason }` for actionable failures.

Read actual Glyphs 2/3, write actual Glyphs 3; preserve native fields, unknown
keys/custom parameters, and supported model extras. Use glyphs-reader plus raw
plist shadow as appropriate. Never substitute a serialized Font for actual
native outline/layout data. External native edits must win over extras. Reject
unapplied feature drafts and imported-layout authority before writing.

Acceptance: fixture-driven native read, native output parsed by glyphs-reader,
source semantic and compiled shaping roundtrips, multiple masters, curves,
components, anchors, kerning groups/exceptions, FEA, unknown data, external edits,
malformed/bounded input, export failure leaves destination intact. Read the
engineering conventions and licensing rules. Run focused tests and Clippy;
report scope and unimplemented cases candidly. Do not commit independently.

## Completion report — 2026-09-19

Implemented native single-file Glyphs 2/3 reading and atomic Glyphs 3 writing.
All masters, native axis locations and mapped user coordinates, glyph ordering,
Unicode, cubic/quadratic/open paths, affine components, anchors, guides,
horizontal/vertical kerning, groups/exceptions and structured FEA are represented
in native fields. Supplemental and Glyphs 2/3 brace layers round-trip. Unknown
source dictionaries, custom parameters, guide/node data, localized descriptions
and metric definitions/values survive the raw source overlay. Narrow editor
metadata retains additional model fields; native edits and deletions win.
Generated-feature identities survive; externally changed generated code freezes
its recipes instead of restoring stale ownership. Readers enforce 64 MiB,
64-level nesting, structural-complexity, finite-number and reference limits.

Verification:

- `rustfmt --edition 2021 --check crates/fc-formats/src/glyphs/*.rs crates/fc-formats/tests/glyphs.rs`: passed.
- `cargo clippy -p fc-formats --all-targets -- -D warnings`: passed.
- `cargo test -p fc-formats --test glyphs`: 14 passed.
- `FC_GLYPHS_SOURCE=<local Yarndings12.glyphs> cargo test -p fc-formats`:
  all 81 crate tests passed, including the real external Glyphs source; doc-tests
  passed. The Glyphs group finished with `14 passed; 0 failed` in 4.90 seconds.
- Independent reviewer ran all 14 with `FC_GLYPHS_SOURCE` pointing at the local
  upstream `soft-type-yarndings-HEAD/sources/Yarndings12.glyphs` (289 KiB, 95 glyphs):
  14 passed, including native Glyphs reader verification, all-outline/metric
  comparisons and compiled shaping comparisons. This external font was not
  copied or redistributed. Separate reviewer reproductions covered mapped axes,
  generated-feature ownership and malformed metrics. All review findings closed.

Explicitly unsupported, with errors: Glyphs packages, smart/corner/cap components,
conditional bracket layers, color/SVG/bitmap/paint behavior, RTL kerning, forced
or partial component alignment, center/right-relative component orientation,
and transforms that cannot be represented accurately by native Glyphs 3 fields.
Existing hint data is retained as source/model metadata; this task does not add
a Glyphs hint interpreter. No FontForge source was adapted. No commit was made.
