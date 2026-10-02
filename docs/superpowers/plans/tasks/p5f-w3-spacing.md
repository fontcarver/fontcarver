# Remaining metrics keyboard increments

Task ID: p5f-w3-spacing

## Scope and ownership
Own app/src metrics/spacing command implementation and focused tests, docs/user relevant section, docs/p5f-w3-spacing-acceptance.md. Discover exact needed files and tell parent before edit; no app/e2e edits (request additions from e2e agent). en.json additive/narrow only if required; never zh-CN.json.

## Interfaces and acceptance
Close wave-1 follow-up for spacing ×100 and exact one-unit keyboard increments after Mod+] conflict fix. Read existing keymaps and metrics behavior; implement nonconflicting accessible binding or commands consistent with existing design, preserve undo and numeric input semantics. Add meaningful regression coverage proving dispatch/conflict behavior and actual amount. Use separate translation pass via parent for any English changes. Focused tests only; request browser verification slot/API8852/UI1462 if needed.

## Shared execution rules
Read AGENTS.md, CONVENTIONS.md, docs/superpowers/plans/tasks/p5f-protocol.md and relevant audit/acceptance docs. The user explicitly authorized all remaining P5 tasks and parallel agents on 2026-10-02; historical “wave 3 not started” statements are superseded. This task is your scope. Shared checkout: no commits, push, stash, reset, clean or unrelated edits. Parent owns STATUS, TODO, feature audit, progress estimate, integration and commits. No recursive subagents needed.
Run artifact checker at start and finish, scratch only .scratch/<task-id>/ with absolute TMPDIR. All Cargo commands use flock .scratch/p5f/cargo.lock, default profiles and shared target. Tell parent before any heavy build or cleaning cache; parent coordinates broad validation. Never delete another task’s artifacts/processes or shared stores. Append gap analysis to this task file before implementation. Focused tests and exact outputs in own acceptance report, distinguish limitations and missing evidence. Final report under 300 words. Stop owned processes and clean owned scratch after preserving acceptance evidence. New translations require a separate GPT-6.1 Sol agent assigned by parent.

## Gap analysis

Audit row 39 advertises LSB/RSB/both ×1/×10/×100/true-1. The existing
`textMetrics.ts` mapper implements base and Shift ×10 bracket steps only;
Mod brackets correctly remain glyph navigation after the wave-1 conflict fix.
The accessible `spacingControls.help` still advertises the removed Mod ×100
and Shift+Mod true-1 bindings, so the visible guidance is also wrong.

Implement Alt brackets for ×100 and Alt+Shift brackets for exactly one unit,
independent of the configured base step, for each selected side and direction.
All Ctrl/Meta bracket combinations remain unclaimed by spacing. Existing
arrow gestures, numeric fields, locked-spacing behavior and the 500 ms undo
coalescing contract remain intact. Reserve all four bracket gestures in the
keymap conflict checker, document them, and update the accessible help through
a separate Chinese translation pass. Prove the mapper, dispatched command
separation, real metric amounts and edit-session grouping with focused tests;
request a real-kernel browser case from the E2E owner.
