# P2b 07 — Remaining static lookup and GDEF/BASE coverage

Date: 2026-09-17. Status: planned, not implemented.
Dependencies: 06.
Read `CONVENTIONS.md`, the [design](../../specs/2026-09-17-p2b-editable-layout.md)
and [implementation plan](../2026-09-17-p2b-editable-layout.md).

## Scope and owned files

`fc-layout` contextual/chaining, marks/cursive, GDEF/BASE writer modules
and coverage fixtures; shared graph interfaces owned by task 05.

## Interfaces and work

Extend coverage to static GSUB types 1–8 and GPOS types 1–9, including class
context/chaining, reverse chaining, alternate/multiple substitution, cursive
and mark-to-base/ligature/mark positioning. Preserve required features,
lookup flags and filtering-set/GDEF dependencies. Handle compiler-representable
GDEF/BASE structures and document unsupported subfields precisely.

Reject unproven contour-point references, variation stores/FeatureVariations,
unsupported FeatureParams/MinMax/device forms and resource-limit hits. Do not
silently drop unreachable data. Locked compiler capability, not merely FEA
syntax in a specification, determines accepted conversion.

## Acceptance tests

Each claimed type/subformat has a synthetic semantic roundtrip. Test nested
lookup references, context/class zero, ignore rules, marks/ligature components,
RTL, script/language differences, stored vertical rules, BASE axes and GDEF
carets/filter sets. Every blocked category has a no-mutation rejection fixture.
Initial simple-Latin coverage alone cannot satisfy this task.

## Verification and handoff

`cargo test -p fc-layout -p fc-compile` with paused timing exclusions;
Clippy; independent kernel review of ordering, filters and support matrix.

Write behavioral failing tests before implementation. Keep scope bounded; report
blocking contract changes to the coordinator. Finish with a report under 300
words: built, commands/results, limitations and unresolved questions. Do not
claim completion from documentation or mock-only tests.
