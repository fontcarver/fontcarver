# P2d-02 SFD interoperability

Implement the SFD section of `../../specs/2026-09-19-p2-completion.md`.
Own `crates/fc-formats/src/sfd/` and `crates/fc-formats/tests/sfd*.rs`.
Public interfaces: `read_sfd(&Path) -> Result<Font, crate::Error>` and
`write_sfd(&Font, &MasterId, &Path) -> Result<(), crate::Error>`.
Parent owns shared registration/dependencies. Use
`Error::Source { format: "SFD", reason }` for actionable failures.

Emit genuine native structures for names, metrics, cubic/quadratic splines,
references/transforms, anchors, kerning and supported layout. Preserve safe
unknown fields with a shadow. Do not use opaque Font metadata to mask native
data loss. Handle text encoding and reject unsupported behavior explicitly.
Reject unapplied feature drafts and imported layout before export. Read official
SFD docs; never copy GPL-only code or execute FontForge. Static SFD is this task;
separate SFDir/bitmap/variable/CID milestones are outside it.

Acceptance: representative external/native fixtures, semantic roundtrips,
external native-field edits, cubic/quadratic and components, Unicode text,
kerning/layout, malformed/resource-bounded input, atomic export failures.
Read conventions/license rules, run focused tests and Clippy, report exact
coverage and blockers. Do not commit independently.

## Completion evidence

Native SFD 1/2/3 and selected-master output are implemented. Independent review
verified kerning source ownership, canonical groups/memberships, keyword glyph
escaping, shadow-header protection, and explicit rejection of unsupported
additive lookup/mkmk anchor semantics. Selected-master export no longer requires
unrelated masters to be complete.

Verification: 1 SFD unit + 15 integration tests passed, including actual optional
DirectionTest/SplineOverlapBug1/QuadOverlapBugs files, independent fontc layout
comparison, and HarfRust kerning/ligature/script tests. Full formats crate and
CLI new-format roundtrip tests passed. Formatting and all-target Clippy passed.
No FontForge implementation was adapted or executed. Supported layout subset and
explicit rejection boundaries are documented in `fc-formats/src/sfd/mod.rs` and
[P2c/P2d acceptance](../../../p2cd-acceptance.md).
