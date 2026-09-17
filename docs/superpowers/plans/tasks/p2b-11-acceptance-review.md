# P2b 11 — P2b corpus, headless acceptance and independent review

Date: 2026-09-17. Status: planned, not implemented.
Dependencies: 01–10 complete.
Read `CONVENTIONS.md`, the [design](../../specs/2026-09-17-p2b-editable-layout.md)
and [implementation plan](../2026-09-17-p2b-editable-layout.md).

## Scope and owned files

Focused/integration tests across affected crates, `app/e2e/p2b*`, licensed
fixtures and `docs/p2b-acceptance.md`; roadmap/current status updates.

## Interfaces and work

Independently verify source drafts, full static coverage, authority transition,
kerning lift fallback, automatic/manual ownership, stale-race behavior, FCF/UFO
roundtrips and TTF/WOFF/WOFF2 exported shaping. Use the full glyph/cluster/offset
comparison from task 06 and exact bytes only for untouched preservation.

Record fixture licenses/hashes and a coverage matrix. Existing real fonts plus
small synthetic edge fixtures are required; optional unavailable external fonts
are explicit skips. Keep per-lookup/corpus claims precise. Test source-only and
lifted paths and inherited P2a behavior. Coordinate fixes with owning tasks.

## Acceptance tests

All design §9 criteria pass; no unresolved correctness/revert/data-loss review
findings. Kernel/frontend reviewers are separate from feature authors.
English/light and Chinese/dark screenshots inspected. No accidental licensing
changes, generated API files or large fixtures are staged.

## Verification and handoff

Run final gate commands from the plan, isolated headless workflows and diff
checks. Report measured counts, warnings/skips and remaining supported boundaries;
only then mark P2b complete. Performance/FontForge/native automation stay paused.

Write behavioral failing tests before implementation. Keep scope bounded; report
blocking contract changes to the coordinator. Finish with a report under 300
words: built, commands/results, limitations and unresolved questions. Do not
claim completion from documentation or mock-only tests.
