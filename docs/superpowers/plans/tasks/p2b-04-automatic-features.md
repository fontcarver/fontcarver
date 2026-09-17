# P2b 04 — Automatic feature source and writer ownership

Date: 2026-09-17. Status: planned, not implemented.
Dependencies: 01 and 03.
Read `CONVENTIONS.md`, the [design](../../specs/2026-09-17-p2b-editable-layout.md)
and [implementation plan](../2026-09-17-p2b-editable-layout.md).

## Scope and owned files

`fc-compile` source preparation/generation, `fc-model` policy types,
`fc-api` generation/freeze integration and focused tests.

## Interfaces and work

Add opt-in deterministic `.sc`, `.ss01`–`.ss20`, `.case`, `.vert` and
unambiguous underscore-ligature recipes. Store explicit script/language/mapping
configuration for `locl` and discretionary ligatures. Materialize name-based
source on apply, with a visible stale-generation status after glyph changes.

Freeze generated source into ordinary editable text and disable its generator
atomically. Old `automatic` metadata does not enable new behavior on load.
Respect locked fontc marker/writer behavior; model-owned kern/anchor generation
and source-owned rules have explicit ownership. Imported conversion begins
without additional name generation or duplicate anchor rules.

## Acceptance tests

Test generation determinism, missing/ambiguous bases, name escaping, user
feature collisions, explicit `locl` languages, freeze/regenerate/undo/reopen,
and changes after glyph rename/add/remove. Verify `# Automatic Code` behavior
through actual compiled layout. Manual and automatic kern/mark rules never
execute twice; P2a ownership gate remains shared by queries and commands.

## Verification and handoff

Focused `fc-model`, `fc-compile`, `fc-api` tests and scoped Clippy; update
schema/tests for any persisted configuration.

Write behavioral failing tests before implementation. Keep scope bounded; report
blocking contract changes to the coordinator. Finish with a report under 300
words: built, commands/results, limitations and unresolved questions. Do not
claim completion from documentation or mock-only tests.
