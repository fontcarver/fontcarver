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

## Gap analysis

Written 2026-09-30 after reading the current code for every row. "Now" is the
state at HEAD `449e753`; "Plan" is what this task implements; "Limit" is what it
records as an explicit boundary.

### Decisions taken without asking

1. **Where project-stored export profiles live.** `FontInfo` cannot gain typed
   fields without breaking the exhaustive `FontInfo { .. }` literals in
   `crates/fc-formats/src/ufo/info.rs` and two test helpers, and `Font` cannot
   gain one without a new `fc-doc` command (a shared file this task may not
   own). Profiles therefore use the project's established namespaced-collection
   pattern — `font.info.user_data["com.fontcarver.inventory"]["export_profiles"]`
   through `handlers::inventory::{read_collection, write_collection}`, exactly
   like saved smart filters and glyph lists. That gives `.fcf` persistence, one
   `SetFontInfo` undo entry and CLI access with **no** shared-file edit, and the
   wire representation stays a typed, ts-rs exported `ExportProfile`.
2. **`crates/fc-api/src/api.rs` is treated as a shared file.** No `p5f-*` task
   file claims it, and every task that adds an API method must append to
   `METHODS` and the dispatch table. Edits there are single-line appends made
   immediately after re-reading the file.
3. **`crates/fc-model/src/layer.rs` (compound bracket conditions) and
   `crates/fc-formats/src/ufo/designspace.rs` (bracket rules) are unowned.**
   Row 56 cannot be implemented without them, so they receive additive-only
   patches: a new optional field on `LayerKind::Bracket` and the matching rule
   writer branch. Both are named in the final report.
4. **Row 57 keeps one implementation.** `GlyphVariationPanel` already mounts the
   axis sliders inside the Edit View inspector and already drives the canvas
   overlay through `variationPreviewStore`. Adding a second `VariationSliders`
   component would duplicate the request/debounce logic, so the existing panel is
   extended (compact slider strip, per-axis reset, named-instance jump, default
   open when the font has axes) instead of replaced. The new file the task file
   suggested is not created; the reason is recorded here.
5. **Per-font grid/increment settings stop at the API and Font Info UI.** The
   editor-side consumers (`GlyphCanvas.tsx`, `editorStore`) belong to task c, so
   this task stores the per-font values, resolves them against `prefs` in the
   kernel response, and exposes them on `doc.font_info`; the one-line editor
   patch is requested in the report.

### Row 52 — scoped info, Family → Font → Master → Export

- **Now.** `metricInheritance.resolveMasterMetric` resolves optional master
  metrics against the compiler defaults and labels the source; the master
  metrics editor shows an override checkbox per field. Instance rows show the
  family fallback. Nothing shows a *family* scope, there is no generic
  "revert to inherited" action outside master metrics, and export-scope
  overrides are limited to `Instance.family_name`/`style_name`.
- **Plan.** A `scopedInfo.ts` resolver plus a `ScopedInfoTable` section in the
  Font Info panel that lists every name/legal/OS-2 field with the scope that
  supplies its effective value (family, font, master, export), an override
  indicator and a revert action. Family scope is the set of values OpenType
  itself defines family-wide (family name, preferred/WWS family, designer,
  manufacturer, license, copyright, trademark, vendor ID, version); font scope
  is this source's own identity; master scope is metrics/zones/stems; export
  scope is per-instance overrides. Export overrides gain
  `weight_class`/`width_class`/`postscript_font_name` through the instance
  `user_data` namespace so no shared model file changes, and
  `fc-compile::instantiate` applies them to the compiled output.
- **Limit.** FontCarver is one document per font; there is no cross-document
  family container, so the family scope is the family-wide field set inside the
  document, not a shared parent file. Recorded, not claimed as more.

### Row 53 — per-master metrics, UPM, grid, increments

- **Now.** Zones, stems and per-master metrics are editable. `FontMetadataEditor`
  edits UPM as a plain number: outlines, advances, kerning and master metrics
  are explicitly *not* scaled (its own help text says so). `grid_size`,
  `grid_subdivision` and `nudge` exist only as application preferences.
- **Plan.** (a) a kernel `variation.rescale_upm` operation that builds one
  atomic `Command::Batch` (scaled contours, components, anchors, advances,
  vertical metrics, master metrics, zones, stems, PostScript private values,
  guides and kerning) with an explicit opt-out per group, reached from the Font
  Info UPM field as "change UPM and scale the drawing"; (b) per-font grid
  spacing, subdivision and the three keyboard increments stored in the project
  and resolved against `prefs` by the kernel, edited in a Font Info "Other
  settings" group with an inherited/overridden indicator.
- **Limit.** The editor's own use of the per-font values is one narrow patch in
  task c's files; this task ships the storage, resolution and UI.

### Row 54 — axes, masters, instances, variable-font configs

- **Now.** `AxesTable`/`InstanceEditor`/`DesignSpaceEditor` edit axes, masters
  and instances. Export takes an ad-hoc `master_subset`, which keeps the axis
  definitions untouched, so a subset that no longer varies an axis still emits
  that axis in `fvar`. There is no named, stored variable-font configuration and
  no instance subsetting.
- **Plan.** Named `VariableExportConfig` records (master subset, axis subset,
  named-instance subset, file name) stored beside the profiles; `fc-compile`
  gains real design-space subsetting — after a master subset each axis is
  clamped to the span the retained masters actually cover and an axis with no
  remaining span is dropped, instances outside the retained span are reported
  and dropped rather than silently kept, and `instance_subset` filters the named
  instances written to `fvar`/`STAT`. Selectable in the export dialog, the
  matrix and the CLI; verified by reading `fvar` back with FontTools.
- **Limit.** Glyphs' per-config `Remove Glyphs`/`Add Feature` parameters are out
  of scope; the config carries the design-space subset and the file name.

### Row 55 — compatibility view

- **Now.** `VariationCompatibility` overlays each source outline in its own
  colour, can list every point with per-point correspondence lines, and calls
  the existing conservative `Repair`. There are no per-shape indices, the colour
  is per *source* rather than per *shape*, there is no green/yellow/red segment
  vocabulary, connectors are per point rather than between corresponding start
  points/shape centres, and shapes cannot be reordered from the view.
- **Plan.** `fc-geom::compatibility` gains a pure `shape_status` analysis
  (per-shape correspondence across sources plus per-segment
  compatible/angle-warning/incompatible status with the 20° rule); the API
  preview carries it; the view draws per-shape colours, shape indices at start
  nodes, start-point and shape-centre connectors and coloured segments, and
  supports dragging one shape onto another to swap shape order through a new
  `DesignSpaceCommand::ReorderShapes` (one undo unit, all sources).
- **Limit.** Anchor connectors are drawn from the existing anchor data only when
  the sources agree on anchor names; FontLab-style node insertion repair stays
  out of scope.

### Row 56 — brace and bracket layers with a real condition editor

- **Now.** `LayerKind::Bracket { master, axis, min, max }` is single-axis.
  `BracketConditionFields` edits exactly one axis range. `fc-compile::bracket`
  and the designspace writer therefore emit single-axis conditions only.
- **Plan.** An additive `conditions: Vec<BracketCondition>` on
  `LayerKind::Bracket` (empty keeps today's single-axis meaning), full
  validation, axis-rename migration, repair keying, `rvrn`/feature-variation and
  designspace-rule export for the compound case, and a condition editor that
  adds and removes per-axis rows and shows the `[80‹wght, 25‹wdth‹50]` label.
- **Limit.** Conditions remain conjunctions of per-axis ranges (the OpenType
  `ConditionSet` model and the Glyphs syntax); disjunctions are expressed by
  adding another bracket layer.

### Row 57 — live axis sliders in Edit View

- **Now.** Implemented and mounted: `EditInspector` → `GlyphVariationPanel` →
  `VariationPreview` sliders → `variationPreviewStore` → `renderGlyph` →
  `drawVariationPreview`. The section is collapsed by default and has no reset.
- **Plan.** Verify the whole loop against the real kernel in the browser, make
  the section open by default for a font with axes, add per-axis reset and a
  keyboard-reachable value field, and keep the interpolation in the kernel.

### Row 58 — profiles in the project, matrix, preflight

- **Now.** Profiles live in `localStorage` (`fontcarver.exportProfiles`), so they
  are neither in the project file nor visible to the CLI — the requirement's
  central claim is unmet. The matrix and preflight components exist and are
  integrated, with component-level evidence only.
- **Plan.** Move profiles into the project as above (document-scoped store,
  24-profile limit kept, subfolder and file-name fields added), let the matrix
  use variable-font configs as rows beside static instances, and produce
  real-kernel browser evidence for saving a profile, running a matrix and
  jumping from a preflight finding to the glyph.

### Row 59 — formats

- **Now.** All six formats exist with earlier acceptance. Not re-verified
  through the current dialog/CLI.
- **Plan.** One reproducible script that drives the CLI for OTF/TTF/WOFF/WOFF2/
  variable TTF/UFO+designspace and checks each output independently with
  FontTools, FreeType and HarfBuzz; browser evidence for the dialog path.

### Row 60 — production post-processing

- **Now.** `add_extrema`, `round_coordinates`, `decompose_components`,
  `production_names`, `remove_overlap` and `subroutinize` all exist as options
  with Rust tests, and the dialog exposes them. Missing: independent
  verification that the *emitted* font shows the effect.
- **Plan.** Extend the acceptance script to assert, with FontTools, that extrema
  appear in `glyf`, that coordinates are integral, that components are gone,
  that `post` carries production names and that the CFF has global subroutines
  and is still readable by FreeType.

### Row 63 — CLI sharing the profiles

- **Now.** `fontcarver build` takes ad-hoc flags. No profile support, no
  preflight report.
- **Plan.** `fontcarver profiles <input>` lists stored profiles and variable
  configs; `fontcarver build --profile <name>` applies the stored options,
  `--variable-config <name>` selects a stored design-space subset,
  `--instances all|<names>` with `--out-dir` runs the same matrix the dialog
  runs, and `--preflight` prints the same `doc.find_problems` report. Covered by
  `assert_cmd` tests.
