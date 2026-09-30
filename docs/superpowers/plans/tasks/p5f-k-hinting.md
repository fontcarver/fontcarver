# p5f-k: Hinting completion

Area id: `k-hinting`. Ports: API 8811, UI 1511. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                                                                    | State at start                                                                                                                                                                                                                                                         |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 61  | Autohinting (ttfautohint for TT, PS autohint from stems/zones)                                                                                 | Partial: static PS analysis and the external ttfautohint bridge work through export UI/CLI (`scripts/p5_hint_acceptance.py`). Variable-font hinting and analysis quality remain open. `ttfautohint` is not installed on this host; keep availability reporting honest. |
| 98  | Manual PostScript hint editing (stems, ghosts, hint replacement)                                                                               | Partial: editor, undo, persistence, CFF stem/mask/counter round trips, Type 1 parameters, seac inheritance, CFF1 per-FD preservation. Flex, dot sections, general counter control, editable per-FD dictionaries and CFF2 private variation remain.                     |
| —   | P5 TT scope: authored TT programs bound to point identities, CVT/maxp, hinted preview, variable-font hint data (`cvar`), imported flex binding | See `docs/p5-hinting-checkpoint.md`, `docs/p5-hinting-resume-checkpoint.md`, `docs/p5-tt-variable-binary-checkpoint.md`.                                                                                                                                               |

Existing records: `docs/p5-hinting-checkpoint.md`, `docs/p5-hinting-resume-checkpoint.md`,
`docs/p5-tt-variable-binary-checkpoint.md`, `docs/p5-cff-private-editor-acceptance.md`,
`docs/acceptance/p5-cff-*.md`, `docs/acceptance/p5-ps-analysis.md`,
`docs/superpowers/plans/tasks/p5-tt-variable-binary.md`, `p5-type1-seac-hints.md`,
`p5-02-cff-flex-guard.md`, `docs/user/hinting.md`, `scripts/p5_hint_acceptance.py`,
`scripts/p5_tt_instruction_acceptance.py`.

## Scope notes

- Row 61 (PS): improve the autohinter to the level expected of a professional
  tool: stems from zones and stem lists, ghost hints at alignment zones, hint
  replacement where stems conflict, counter control for `hstem3`/`vstem3`
  patterns, and a quality gate that compares against the font's existing hints
  on the fixture corpus (FreeType renders at small ppem should not regress in
  a measured way; record the metric). Variable fonts: CFF2 has no hints, so
  document that PS autohint applies to static instances; for TT variable
  fonts, either implement `cvar`-aware CVT authoring or record the exact limit.
- Row 61 (TT): the external ttfautohint bridge must detect availability,
  pass options through, capture warnings and never claim success when the tool
  is absent; add a test that simulates absence and one that runs when the
  binary exists (skipped otherwise). Document installation in `docs/user/hinting.md`.
- Row 98: flex hints (editable flex on two curve segments, Type 1 and CFF
  `flex`/`flex1` output), `dotsection`, general counter control groups, editable
  per-FD private dictionaries for CID-keyed CFF (not only preservation), and
  CFF2 private-dictionary variation data preserved through import/export.
  Every editing path: undo, `.fcf` persistence, export verified with FontTools
  (`T2CharString` decompile) and FreeType rendering.
- TT: authored programs keep binding to point identities through edits;
  hinted preview at several ppem in the panel; `cvar` and per-master CVT values
  for variable fonts if the P5 checkpoint left them open; validated compiled
  bindings with independent interpreter tests (FreeType with hinting enabled).

## Owned files

`crates/fc-hint/**`; `crates/fc-sfnt/src/{write/hints.rs,write/cff.rs,write/cff2.rs,write/charstring.rs,tt/**,cff_interchange.rs}`;
`crates/fc-model/src/{ps.rs,tt.rs}`; `crates/fc-doc/src/command/hints.rs`;
`crates/fc-api/src/handlers/{hints.rs,tt_hints.rs}`, `crates/fc-api/src/types/{hints.rs,tt_hints.rs}`;
`crates/fc-raster/src/hint.rs`; `app/src/views/hints/**`;
`crates/fc-formats/src/type1/**` (hint parts); `scripts/p5_hint_acceptance.py`,
`scripts/p5_tt_instruction_acceptance.py`; `docs/user/hinting.md`; `docs/p5f-k-hinting-acceptance.md`.

Shared files (narrow patches only): `crates/fc-compile/src/**` (hint pipeline
hooks; task f owns the crate), `crates/fc-sfnt/src/{write/mod.rs,read/**}`,
`crates/fc-model/src/glyph.rs`, `crates/fc-doc/src/fcf/schema.md`,
`app/src/dialogs/ExportProcessingOptions.tsx`, `app/src/i18n/en.json`,
`crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`.
