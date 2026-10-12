# P2b 05 — Imported layout inventory and lookup graph

Date: 2026-09-17. Status: implemented and independently reviewed; final P2b acceptance passed.
Dependencies: 01; coordinate API conventions with 03.
Read `CONVENTIONS.md`, the [design](../../specs/2026-09-17-p2b-editable-layout.md)
and [implementation plan](../2026-09-17-p2b-editable-layout.md).

## Scope and owned files

`fc-layout` import/graph/capability modules; `fc-sfnt/src/read/mod.rs`;
`fc-api` layout query types/handlers; synthetic fixtures.

## Interfaces and work

Introduce runtime `LayoutImport` keyed by raw table hash and glyph provenance:
owned parsed tables, ordered lookup graph, rendering/coverage state. Keep raw
bytes/provenance as the persistent source; no duplicate binary payload in FCF.
Implement paged `layout.inspect`/`layout.rules` (1–200 items, bounded per-item
payload, generation checked); expose coverage without implying editability.

Inventory GSUB/GPOS/GDEF/BASE, BASE-only documents, legacy kern/AAT competitors
and JSTF lookup-index dependencies. Old FCF contradictions are blocked for
conversion. Validate offsets, glyph references, recursion/cycles and resource
limits before traversal; unsupported content stays preserved and inspectable.

## Acceptance tests

Tests cover missing tables, BASE-only and legacy kern-only import, old FCF,
malformed offsets, missing GIDs, shared lookups, cycles, extensions, resource
limits and paged/stale reads. JSTF and competing layouts produce explicit
conversion blockers. Inventory never changes authority or dirty state.

## Verification and handoff

`cargo test -p fc-layout -p fc-sfnt -p fc-api`; focused malformed-font tests,
Clippy, API generation and TypeScript check.

Write behavioral failing tests before implementation. Keep scope bounded; report
blocking contract changes to the coordinator. Finish with a report under 300
words: built, commands/results, limitations and unresolved questions. Do not
claim completion from documentation or mock-only tests.

## API implementation evidence — 2026-09-17

`layout.inspect`, `layout.rules` and `layout.rule_glyphs` expose imported tables,
ordered lookup metadata, explicit coverage blockers, source-like rule renderings
and stable session glyph handles. Pages require limits from 1 through 200;
continuations require the original document generation. Responses carry document,
compile and source/draft revisions plus the imported-layout fingerprint.

The runtime inventory is parsed outside the session lock and cached for at most
eight documents, keyed by effective compile revision. Close clears its cache.
Every response rechecks the complete snapshot identity before binding glyph
handles. Inspection never mutates authority, history or dirty state. A converted
native backup is identified separately from conversion coverage.

Rule previews contain at most 4096 characters and 64 glyph references; the
separate glyph-reference endpoint pages the complete bounded class/reference
list without expanding pairs. An actual 70-alternate GSUB fixture verifies that
references after the first 64 remain available through a continuation page.

API verification: the initial two `layout_inspection` integration tests passed
for bounded/read-only pages, stable glyph handles and stale continuations; the
additional large-alternate-class regression passed independently. Generated
TypeScript and scoped all-target Clippy passed. Kernel evidence below covers
malformed binary traversal and graph coverage; the final integrated acceptance
also passed (see [P2b acceptance](../../../p2b-acceptance.md)).

## Kernel implementation evidence — 2026-09-17

`LayoutImport` hashes raw layout tables and original glyph order, retains checked
lookup graphs only at runtime, and bounds all inspector output. BASE-only,
legacy-only/AAT/JSTF authority, native conversion backup inspection, malformed
references, cycles and allocation amplification have regression fixtures.
FeatureParams or registration blockers retain decoded supported lookup rules
and paged glyph references for inspection while whole conversion stays blocked.
`layout_inventory` has 8 passing integration tests; the licensed NotoSans blocked
inspection regression also passes. See the capability matrix and task 07 for
additional decompiler and canonical-equivalence evidence.
