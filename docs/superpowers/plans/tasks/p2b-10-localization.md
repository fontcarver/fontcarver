# P2b 10 — Independent Chinese translation and localization QA

Date: 2026-09-17. Status: independently translated and verified.
Dependencies: 09 English keys and product workflows stable.
Read `CONVENTIONS.md`, the [design](../../specs/2026-09-17-p2b-editable-layout.md)
and [implementation plan](../2026-09-17-p2b-editable-layout.md).

## Scope and owned files

`app/src/i18n/zh-CN.json`; localization QA report. Translator is a separate
agent from Features UI authors, as required by CONVENTIONS.md.

## Interfaces and work

Translate feature source, drafts, automatic generation, diagnostics, lookup
coverage and conversion/revert actions consistently with existing P2a wording.
Preserve interpolation, pluralization and keys. Use product language for
unsupported spacing behavior; do not expose Rust enum names as instructions.
Report any unclear English copy to the author before final parity verification.

## Acceptance tests

English/Chinese key and placeholder parity, no hard-coded new user strings,
Chinese/dark visual check for tree/diagnostics/action labels and overflow.
Keyboard/accessible names remain translated and reachable.

## Verification and handoff

Localization parity checks; relevant component tests; record changed keys,
terminology and visual evidence without claiming production-wide translation review.

Write behavioral failing tests before implementation. Keep scope bounded; report
blocking contract changes to the coordinator. Finish with a report under 300
words: built, commands/results, limitations and unresolved questions. Do not
claim completion from documentation or mock-only tests.

## Completion evidence

A separate translator added 144 Simplified Chinese keys, including recovery,
conflict and freeze guidance. Both catalogs have 1,115 keys with matching
interpolation/plural forms; localization checks and formatting pass.
Chinese/dark source and lookup screenshots were inspected alongside
English/light source. Runtime compiler diagnostics remain available as
technical details; primary conversion outcomes and controls are translated.
See [P2b acceptance](../../../p2b-acceptance.md).
