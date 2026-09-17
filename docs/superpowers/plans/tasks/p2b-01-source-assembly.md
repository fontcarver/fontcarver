# P2b 01 — Mapped FEA assembly and UFO compatibility

Date: 2026-09-17. Status: planned, not implemented.
Dependencies: None beyond completed P2a.
Read `CONVENTIONS.md`, the [design](../../specs/2026-09-17-p2b-editable-layout.md)
and [implementation plan](../2026-09-17-p2b-editable-layout.md).

## Scope and owned files

`crates/fc-model/src/features.rs`, new mapped-assembly module if needed;
`crates/fc-formats/src/ufo/structure.rs`; relevant `fc-compile`/`fc-doc`
assembly call sites and focused tests.

## Interfaces and work

Add a pure assembled-source value with text and source-item byte ranges.
Keep `Features::to_fea()` as a compatibility wrapper. Introduce stable source
item IDs with backward-compatible defaults/migration; map generated wrappers
to their enclosing item. Preserve spelling, comments and ordering.

Import whole-file UFO FEA as a prefix. Normalize only the known historical
synthetic `fea` feature representation, preserving disabled state and content.
Use the same source assembly on UFO write and compiler/eligibility paths.
Do not add draft persistence, editor UI or decompilation in this task.

## Acceptance tests

First add a failing UFO test with `languagesystem`, a class, lookup and ligature
feature before implementation. Test full-file and body-only feature blocks,
classes with/without brackets, disabled blocks, duplicate tags and old `fea`
records. Compare source text and compiled/reimported shaping; test source maps
for Chinese comments, emoji and CRLF. Existing P2a eligibility tests stay green.

## Verification and handoff

`cargo test -p fc-model -p fc-formats`; focused `fc-compile` feature-source and
`fc-doc` eligibility tests; scoped Clippy and workspace format check.

Write behavioral failing tests before implementation. Keep scope bounded; report
blocking contract changes to the coordinator. Finish with a report under 300
words: built, commands/results, limitations and unresolved questions. Do not
claim completion from documentation or mock-only tests.
