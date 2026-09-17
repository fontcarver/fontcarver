# P2b 04 — Automatic feature source and writer ownership

Date: 2026-09-17. Status: implemented and independently reviewed; final P2b acceptance passed.
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

## Implementation evidence — 2026-09-17

- `FeatureGenerationSettings` keeps recipes opt-in, writer ownership explicit,
  generated source IDs separate from legacy automatic metadata, and a persisted
  exported-name fingerprint for stale generation. Both `locl` and `dlig` store
  explicit script/language mappings; language tag padding is canonicalized for
  duplicate checks. Native mappings round-trip; missing dlig scope fields use
  `DFLT`/`dflt`.
- `generate_feature_source` deterministically materializes stored source and
  preserves manual/frozen items. `freeze_feature_source` disables one recipe
  without regenerating the visible snapshot. Missing/ambiguous bases, unsafe
  names, duplicate mappings and user-owned feature collisions are diagnosed.
- The actual fontc pipeline uses explicit Skip writer policies. Source-owned
  conversion can suppress model kern/mark/curs/GDEF while preserving model data.
  A manual rule plus `# Automatic Code` and an enabled competing writer fails
  explicitly; direct compilation applies the same fail-closed ownership check.
- Shaping tests verify kerning/mark insertion exactly once, retained anchors,
  Turkish versus English `locl`, Turkish Latin versus Serbian Cyrillic `dlig`,
  disabled feature behavior, and absence of cross-language substitution leaks.
  A warning-only manual source regression proves regeneration retains existing
  automatic rules and still checks collisions.

Compiler-focused verification: 20 tests passed across `feature_generation`,
`feature_validation`, and `feature_writers`; 2 model generation/serialization
tests passed. `cargo clippy -p fc-model -p fc-compile --all-targets -- -D warnings`,
scoped formatting, and `git diff --check` passed. Full workspace/API/UI gates
passed; see [P2b acceptance](../../../p2b-acceptance.md).
