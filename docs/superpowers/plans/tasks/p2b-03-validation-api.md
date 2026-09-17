# P2b 03 — Structured diagnostics and guarded activation

Date: 2026-09-17. Status: planned, not implemented.
Dependencies: 01 and 02.
Read `CONVENTIONS.md`, the [design](../../specs/2026-09-17-p2b-editable-layout.md)
and [implementation plan](../2026-09-17-p2b-editable-layout.md).

## Scope and owned files

`fc-compile` validation/diagnostics; `fc-api/src/types/features.rs`,
`handlers/features.rs`, dispatch/generator; trusted `fc-doc` integration;
`fc-formats` source-export guards and API/CLI regression tests.

## Interfaces and work

Implement `features.get`, `set_draft`, `discard_draft`, `validate` and `apply`
contracts from the spec. Parsing can serve live diagnostics; activation requires
actual fontc compilation for every affected master with the same writer policy
as export. Take snapshots under the document lock, compile outside it, and
recheck generation/source identity before committing.

Return bounded diagnostics by source item, severity, code, master and UTF-16
span. Includes fail closed without filesystem access. Apply failure and stale
results never mutate authority/history. Reject all non-FCF exports with unapplied
changes; fresh glyph changes require fresh validation of accepted source.
Diagnostic requests/results also carry effective compile-input revision;
unchanged draft text does not make results current after glyph/metadata edits.
Even without a draft, imported-table authority rejects UFO/designspace source
export until conversion; the writer itself checks before touching a destination.
Generated TypeScript only; no caller-supplied proof or authority enum override.
`features.apply` rejects imported authority; only the validated baseline
conversion path from task 08 can switch an imported document to source.

## Acceptance tests

Tests cover syntax versus semantic failures, generated-wrapper locations,
Unicode spans, invalid/master-specific source, includes, stale concurrent edits,
closed documents, atomic no-op/failure and successful apply/undo. Try unchecked
`doc.apply`/forged payloads. Native save allows an invalid draft; binary/UFO
export rejects unapplied text. Existing P2a exported shaping remains correct.
Imported source export rejects clean/unsupported originals with destination
unchanged, including direct source-writer and CLI entry points.
Delayed validation across glyph deletion and undo/redo must not display stale
success merely because the draft revision stayed unchanged.

## Verification and handoff

`cargo test -p fc-compile -p fc-api -p fc-doc`; scoped Clippy; `pnpm --dir app
gen:api`, `pnpm check`; API fixtures/documentation agree.
Run focused `fc-formats`/CLI export-guard tests as well; the direct source-writer
boundary must be verified independently of the API handler.

Write behavioral failing tests before implementation. Keep scope bounded; report
blocking contract changes to the coordinator. Finish with a report under 300
words: built, commands/results, limitations and unresolved questions. Do not
claim completion from documentation or mock-only tests.
