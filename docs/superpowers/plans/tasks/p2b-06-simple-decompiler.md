# P2b 06 — Simple static layout decompiler and equivalence harness

Date: 2026-09-17. Status: implemented and independently reviewed; final P2b acceptance passed.
Dependencies: 03 and 05.
Read `CONVENTIONS.md`, the [design](../../specs/2026-09-17-p2b-editable-layout.md)
and [implementation plan](../2026-09-17-p2b-editable-layout.md).

## Scope and owned files

`fc-layout` FEA writer/simple GSUB/GPOS modules, comparison helpers,
synthetic fixture builders and tests.

## Interfaces and work

Emit deterministic named lookups/classes for GSUB single/ligature, GPOS
single/pair, extension wrappers and GDEF classes. Preserve feature/script/
language registration, shared references, lookup/subtable order, flags and
class-zero semantics. Any unsupported subfield blocks whole-document conversion.

Build canonical structure/rule comparison and a shaping helper reporting glyph
names, clusters, advances and offsets with script/language/direction/feature
values. Compile through the actual fontc candidate pipeline; avoid comparing
GIDs blindly or treating a few Latin strings as proof. No UI or authority flip.

## Acceptance tests

Fixtures exercise PairPos formats 1/2, class zero, explicit zero masking,
nonzero and encoded-zero second values, multiple subtables, shared lookup
references, extension wrappers and script/language differences. Negative
fixtures prove no silent loss. Compare before/after using identical outline
and glyph-order baselines, then full shaping records.

## Verification and handoff

Focused `fc-layout` and `fc-compile` tests, scoped Clippy; checked-in capability
matrix names every supported/rejected format in this slice.

Write behavioral failing tests before implementation. Keep scope bounded; report
blocking contract changes to the coordinator. Finish with a report under 300
words: built, commands/results, limitations and unresolved questions. Do not
claim completion from documentation or mock-only tests.

## Implementation evidence — 2026-09-17

Implemented in `fc-layout/src/import/`: checked binary readers, ordered lookup
and feature graphs, complete static decompiler, bounded inspector records and
whole-graph equivalence. GSUB 1–8 / GPOS 1–9 and static representable GDEF/BASE
have actual pinned-compiler roundtrips, including all twelve GSUB/GPOS context
format combinations. Refer to
[`LAYOUT_CAPABILITIES.md`](../../../../crates/fc-layout/tests/LAYOUT_CAPABILITIES.md)
for exact accepted fields, rejection fixtures, resource bounds and real-font
corpus outcomes. Unsupported fields and unproven recompilation remain blocked;
no conversion is authorized by shaping samples alone.

Focused validation: 48 import unit tests passed; 10 layout import and 8 inventory
integration tests passed together, then the added blocked-font inspector case
and two full GDEF/BASE graph cases passed separately. Logs are in ignored
`target/p2b-import-{unit,integration,inspector,full-graph}.log`; the final milestone
report records consolidated suite counts (see [P2b acceptance](../../../p2b-acceptance.md)). Separate kernel review findings were
fixed with regressions for lookup sharing, empty language fallback, ordering and
shared-offset allocation limits.
