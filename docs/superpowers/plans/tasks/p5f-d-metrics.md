# p5f-d: Metrics, spacing and kerning

Area id: `d-metrics`. Ports: API 8804, UI 1504. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                                         | State at start                                                                                                                                                     |
| --- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 37  | Metrics keys with the full syntax (`=n`, `=n±k`, `=\|n`, `=50`, `==n` local, `=n@y`, `=+k` on auto-aligned)         | Reference/offset/mirror/literal exist; local `==`, `@y` height and `=+k` on auto-aligned components to verify/implement. See `docs/p5-metrics-keys-acceptance.md`. |
| 38  | Out-of-sync indicators + Update Metrics (current / all masters)                                                     | Current-layer behaviour exists; all-masters update to verify.                                                                                                      |
| 39  | Spacing shortcuts with LSB / RSB / both, ×1 / ×10 / ×100 / true-1                                                   | LSB/RSB exist; both sides, true-1 and modifier grammar to verify.                                                                                                  |
| 40  | Kerning modes: off / on / locked (kern without risk of respacing)                                                   | `kerningModes.ts`, `KerningModeControl`; locked mode to verify.                                                                                                    |
| 41  | Kerning groups per side; class-to-class by default; modifier-held class suppression for exceptions                  | Groups exist; modifier-held exception creation to verify.                                                                                                          |
| 42  | Kerning colour coding in canvas (negative vs positive) and group-member ghosting                                    | `kerningGhosts.ts`, `KerningGhostControls`; verify colours and ghosts.                                                                                             |
| 43  | Kerning table: sortable, filterable, group rename with scope prompt, Clean Up, Compress, copy/paste between masters | `KerningTable`, `KerningRenameDialog`, `kerningTableData`; see `tasks/p5-kerning-maintenance*.md`.                                                                 |
| 44  | Spacing tab with editable metrics matrix, beam measurement, cut-off display, saved proof strings                    | `SpacingControls`; matrix, beam, cut-off and saved strings to verify/implement.                                                                                    |
| 45  | [CJK] Vertical kerning and vertical metrics editing                                                                 | Reusable evidence (P4, `docs/p5-cjk-vertical-acceptance.md`); re-verify.                                                                                           |

Existing records: `docs/p5-metrics-keys-acceptance.md`, `docs/p5-metrics-ui-acceptance.md`,
`docs/p5-cjk-vertical-acceptance.md`, `docs/p2a-acceptance.md`,
`docs/superpowers/plans/tasks/p5-kerning-maintenance*.md`, `p5-metrics-ui.md`,
`p5-metrics-afm*.md`.

## Scope notes

- Row 37: implement every listed form in the parser, resolver and the UI
  field with inline validation and a tooltip explaining the syntax; `==n` is
  local to the current layer; `=n@y` measures the sidebearing at height `y`;
  `=+k` applies on components auto-aligned via anchors; cyclic keys are
  detected and reported, never looped.
- Row 38: out-of-sync badge in the cell (coordinate with task b: it reads
  the status you expose through the API), the metrics panel and the Spacing
  tab; Update Metrics for the current master and for all masters, each one
  undo unit.
- Row 39: keyboard grammar for LSB, RSB and both sides at ×1/×10/×100 and a
  true-1 (ignores rounding/grid) step, documented in the shortcuts doc.
- Row 40: locked mode kerns without ever changing sidebearings even when the
  gesture would otherwise respace; the mode is visible in the text bar.
- Row 41: class-to-class default with a held modifier producing a glyph
  exception; exceptions are visually marked in canvas and table.
- Row 43: sortable/filterable table over pair, groups, value and master;
  rename with a prompt whether to rename in one master or all; Clean Up removes
  zero/redundant pairs; Compress folds pairs into class pairs where safe;
  copy/paste kerning between masters with a preview of the count.
- Row 44: the Spacing tab shows an editable matrix (LSB, RSB, advance, keys) for
  the glyphs in the text run, a draggable beam that measures at a height, a
  cut-off display (show only the spacing zone), and saved proof strings
  (project-stored, coordinate with the proof library file owned by task h:
  use the existing `proofLibrary.ts` store API without editing it).

## Owned files

`app/src/views/text/{kerningEdits.ts,KerningGhostControls*,kerningGhosts.ts,kerningImpact.ts,KerningModeControl.tsx,kerningModel.ts,kerningModes.ts,kerningQuery.ts,KerningRenameDialog.tsx,kerningTableData.ts,KerningTable*,SpacingControls*,TextGapPicker*,textGaps.ts,drawTextGaps.ts,textMetrics.ts,TextKerningPanel*,useTextKerning.ts,useTextMetrics.ts}` and tests;
`app/src/views/glyph/panels/{KerningGroup*,kerningGroup*,useKerningGroupQuery.ts,MetricsPanel.tsx,metricsKeyCommands.ts,useResolvedMetrics.ts,VerticalMetricsPanel.tsx}`;
`app/src/views/glyph/{drawMetrics.ts,metricsHandles.ts}`;
`crates/fc-doc/src/{metrics.rs,kerning/**,kerning.rs,command/metrics.rs,command/kerning.rs}`;
`crates/fc-model/src/kerning.rs`; `crates/fc-api/src/handlers/kerning.rs`, `crates/fc-api/src/types/kerning.rs`;
`app/e2e/text.spec.ts`, `app/e2e/p2a-kerning.spec.ts`; `docs/p5f-d-metrics-acceptance.md`;
`docs/keyboard-shortcuts.md` (spacing section only); user docs for spacing/kerning.

Shared files (narrow patches only): `app/src/views/text/{TextView.tsx,TextBar.tsx,textLayout.ts}`,
`crates/fc-model/src/{glyph.rs,master.rs}`, `crates/fc-doc/src/fcf/schema.md`,
`crates/fc-api/src/handlers/{glyphs.rs,glyph_status.rs}`, `app/src/shell/{builtinCommands,shortcuts}.ts`,
`app/src/i18n/en.json`, `crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`.
