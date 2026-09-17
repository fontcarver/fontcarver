# P2b 08 — Atomic imported conversion, revert and kerning/anchor lift

Date: 2026-09-17. Status: planned, not implemented.
Dependencies: 02–07; first simple workflow may integrate after 06 behind coverage gates.
Read `CONVENTIONS.md`, the [design](../../specs/2026-09-17-p2b-editable-layout.md)
and [implementation plan](../2026-09-17-p2b-editable-layout.md).

## Scope and owned files

`fc-layout` proof/lift logic; `fc-compile/src/preserve.rs`; `fc-doc`
conversion snapshots; `fc-api` prepare/apply/revert handlers.

## Interfaces and work

Implement prepare/apply conversion as validated, document-scoped server tokens
with bounded lifetime/count. Complete all four layout-table ownership decisions;
compiled absence must not fall back to stale raw tables. Block competitors,
JSTF and unsupported fields; compile outside locks and recheck input identities.

Keep source conversion independent of optional model lift. Start lift with one
exclusive noncontextual kern PairPos lookup, XAdvance-only Value1 and true
ValueFormat2=0. Preserve flags, script assignment, class complements, subtable
precedence and zero rules through actual fontc compilation. No Cartesian pair
expansion or naive summation of lookups. Failed lift leaves FEA-owned kerning.

Also implement whole-lookup mark-to-base/mark-to-mark lifting into named
per-master anchors when static XY coordinates, mark classes, ordering, flags
and script/language assignments survive actual writer compilation. Reject
conflicting model anchors, shared/contextual callers, unproven point/device or
ligature/cursive mappings. Keep unsupported anchors in source. Snapshot affected
glyph/layer/anchor identities without outline-touch invalidation; delete FEA
lookups only together with installation of equivalent model anchors.

Revert restores authority plus only conversion-owned model data, preserving
outlines/metadata. Persist enough provenance for FCF reopen; reject unsafe
glyph mappings/missing raw data. Undo restores replaced source/spacing edits.

## Acceptance tests

Tests cover success/failure/stale/forged token, close/reopen invalidation,
compiler failures, all-or-nothing snapshots, no raw-table resurrection,
source-only fallback, mark flag/script mismatches and class-zero/zero-value
lift cases. Test edit→FCF save→reopen→revert→undo, binary export/reopen shaping,
original byte preservation,
unsafe changed glyph order and no outline/hinting invalidation from conversion.
Require positive mark and mkmk lift fixtures plus a rejected-lift source fallback;
test anchor editing, export, FCF reopen and exact undo/revert restoration.

## Verification and handoff

Relevant `fc-doc`, `fc-layout`, `fc-compile`, `fc-api` integration suites,
Clippy and generated API checks; independent ownership/proof review.

Write behavioral failing tests before implementation. Keep scope bounded; report
blocking contract changes to the coordinator. Finish with a report under 300
words: built, commands/results, limitations and unresolved questions. Do not
claim completion from documentation or mock-only tests.
