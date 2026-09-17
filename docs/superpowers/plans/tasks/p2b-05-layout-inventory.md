# P2b 05 — Imported layout inventory and lookup graph

Date: 2026-09-17. Status: planned, not implemented.
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
