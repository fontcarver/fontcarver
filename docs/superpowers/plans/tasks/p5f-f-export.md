# p5f-f: Font info, design space, export and CLI

Area id: `f-export`. Ports: API 8806, UI 1506. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                                                | State at start                                                                                                                                                                   |
| --- | -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 52  | Scoped info: Family → Font → Master → Export with inheritance and override indicators                                      | `metricInheritance.ts`, `docs/p5-scoped-info-acceptance.md`; full four-level inheritance and indicators to verify.                                                               |
| 53  | Per-master metrics, alignment zones, stems; UPM, grid spacing & subdivision, keyboard increments                           | `MasterMetricsEditor`; UPM change (with scaling prompt), grid/subdivision and increments to verify.                                                                              |
| 54  | Axes, masters at coordinates, static instances, ≥1 variable-font config with designspace subsetting                        | `AxesTable`, `InstanceEditor`, `DesignSpaceEditor`; persistent variable-font export configs to verify/implement.                                                                 |
| 55  | Compatibility view: shape-order colours & indices, cross-master connectors, green/yellow/red segments, drag-to-swap repair | `VariationCompatibility*`, `fc-geom/compatibility.rs`; colours, indices, connectors, statuses, drag-to-swap to verify.                                                           |
| 56  | Intermediate (brace) and Alternate (bracket) layers with a proper condition editor                                         | `BracketConditionFields`, `bracketConditions.ts`; compound conditions to verify/implement.                                                                                       |
| 57  | Live axis sliders scrubbing the drawing in Edit View                                                                       | `variationPreviewStore`, `drawVariationPreview.ts`; live sliders in the Edit View to verify/implement.                                                                           |
| 58  | Export profiles stored in the project; instance × profile matrix; preflight report with click-to-jump                      | `ExportProfiles`, `ExportMatrixPanel`, `ExportPreflight`; see `docs/p5-export-preflight-acceptance.md`, `docs/p5-export-ui-acceptance.md`; real-kernel browser evidence missing. |
| 59  | Formats: OTF (CFF), TTF, WOFF2, WOFF, variable TTF, UFO + designspace                                                      | Reusable evidence; re-verify through the current export dialog and CLI.                                                                                                          |
| 60  | Remove overlap, extrema insertion, coordinate rounding, production names, decompose, subroutinize                          | Overlap/rounding/names/decompose exist; extrema insertion and subroutinization as export options to verify/implement.                                                            |
| 63  | CLI sharing the same profiles for CI                                                                                       | Shared compiler; CLI reading project-stored profiles to verify/implement.                                                                                                        |

Existing records: `docs/p5-scoped-info-acceptance.md`, `docs/p5-font-metadata-acceptance.md`,
`docs/p5-export-ui-acceptance.md`, `docs/p5-export-preflight-acceptance.md`,
`docs/superpowers/plans/tasks/p5-58-export-workflows-acceptance.md`,
`docs/superpowers/plans/tasks/p5-remaining-formats*.md`, `docs/interchange-formats.md`,
`docs/p4-acceptance.md`, `docs/p2cd-acceptance.md`, `docs/p5-type42-acceptance.md`.

## Scope notes

- Row 52: every info field shows where its value comes from (family, font,
  master, export) with an override indicator and a "revert to inherited"
  action; export (instance/profile) overrides apply in the compiled output.
- Row 53: changing UPM offers to scale outlines/metrics/kerning; grid spacing
  and subdivision and the keyboard increment set are per-font settings used by
  the editor (coordinate with task c through the prefs/font-info API only).
- Row 54: named variable-font export configurations (axis subset, instance
  subset, file name) stored in the project and selectable in the export dialog
  and CLI; subsetting verified with FontTools on the output.
- Row 55: per-shape index and colour consistent across masters; connectors
  between corresponding start points; green/yellow/red segment status; drag a
  shape onto another to swap order, one undo unit; verify with a real
  incompatible fixture.
- Row 56: condition editor supports compound conditions (multiple axes,
  ranges) for bracket layers and coordinates for brace layers; export to
  designspace rules/`rvrn` verified with FontTools.
- Row 57: axis sliders in the Edit View inspector (new component under
  `app/src/views/glyph/panels/VariationSliders*.tsx`) drive an interpolated
  preview overlay of the current glyph while editing; mount with a narrow
  patch and keep the interpolation in `fc-geom`/the API.
- Row 58: browser evidence against the real kernel for saving a profile,
  running an instance × profile matrix and jumping from a preflight issue to
  the glyph.
- Row 60: extrema insertion and CFF subroutinization as export options with
  FontTools verification of the output (`fc-sfnt/write/subroutinize.rs` exists).
- Row 63: `fontcarver build --profile <name>` uses the project's stored
  profiles and prints the same preflight report; test with `assert_cmd`.

## Owned files

`app/src/views/fontinfo/**` except `Cid*`, `cidSession.ts` (task l) and `VirtualMastersEditor.tsx` (task j);
`app/src/views/glyph/panels/{GlyphVariationPanel.tsx,BracketConditionFields.tsx,bracketConditions.ts}`,
`app/src/views/glyph/drawVariationPreview.ts`, new `app/src/views/glyph/panels/VariationSliders*`;
`app/src/dialogs/{ExportDialog.tsx,exportFormats.ts,ExportFormatTabs.tsx,ExportMatrixPanel.tsx,exportMatrix.ts,exportOptionState.ts,ExportPreflight.tsx,ExportProcessingOptions.tsx,ExportProfiles.tsx,ExportResult.tsx,ExportVariationOptions.tsx}` and tests;
`app/src/stores/{exportProfileStore,variationPreviewStore,fontInfoCache}*`;
`crates/fc-compile/**`; `crates/fc-cli/**`; `crates/fc-api/src/handlers/{export.rs,variation.rs}`;
`crates/fc-api/src/types/{export.rs,variation.rs,font.rs}`;
`crates/fc-model/src/{designspace.rs,master.rs,info.rs}`; `crates/fc-doc/src/command/designspace.rs`;
`crates/fc-geom/src/{compatibility.rs,interpolate.rs,variation.rs}`;
`crates/fc-sfnt/src/write/subroutinize.rs`; `app/e2e/export.spec.ts`;
`docs/p5f-f-export-acceptance.md`; `docs/interchange-formats.md`; user docs for export and design space.

Shared files (narrow patches only): `crates/fc-model/src/{font.rs,glyph.rs}`,
`crates/fc-doc/src/fcf/schema.md`, `app/src/views/glyph/{GlyphView.tsx,panels/EditInspector.tsx}`
(mount point for the sliders only), `app/src/shell/builtinCommands.ts`,
`app/src/dialogs/DialogHost.tsx`, `app/src/i18n/en.json`,
`crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`,
`crates/fc-sfnt/src/write/mod.rs`.
