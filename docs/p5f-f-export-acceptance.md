# p5f-f acceptance: font info, design space, export and CLI — 2026-09-30

Area `f-export`, audit rows 52–60 and 63. Task file:
[`p5f-f-export.md`](superpowers/plans/tasks/p5f-f-export.md), whose
`## Gap analysis` section records the decisions this report refers to. Protocol:
[`p5f-protocol.md`](superpowers/plans/tasks/p5f-protocol.md).

## What changed

**Export profiles became project data (rows 58, 63).** Profiles used to live in
`localStorage`, so they were neither in the project file nor visible to the
command line — the requirement's central claim was unmet. They are now stored in
the document under `font.info.user_data["com.fontcarver.inventory"]`, beside the
saved smart filters and glyph lists, through `export.profiles_get` /
`export.profiles_set`. That gives `.fcf` persistence, one ordinary `SetFontInfo`
undo entry and command-line access. A profile carries a name, an output
subfolder, an output base name and the full `ExportOptions` record.
`export.resolve_profile` turns one profile plus one matrix row into concrete
options and a relative path; the export dialog, the matrix and
`fontcarver build --profile` all call it, so they cannot drift.

**Design-space subsetting became real (row 54).** Named
`VariableExportConfig` records (master subset, axis subset, named-instance
subset, file name) are stored beside the profiles. `fc-compile` now clamps each
axis to the span its retained masters cover, drops an axis whose masters no
longer differ, drops named instances outside the retained space, trims
alternate-layer condition ranges on a dropped axis and drops an alternate that
loses every axis — each with an export warning, never silently.

**Compound alternate-layer conditions (row 56).** `LayerKind::Bracket` gained an
additive `extra: Vec<BracketCondition>` field, so a condition is now a set of
per-axis ranges evaluated as a conjunction — the OpenType `ConditionSet` model
and the meaning of the Glyphs `[80‹wght, 25‹wdth‹50]` notation. The canonical
(axis-sorted) set is the identity of an alternate _region_; interpolation,
expansion, repair, `rvrn` feature variations, designspace rules and the Glyphs
`axisRules` writer all use it. `DesignSpaceCommand::Conditions` moves every
source of one region to a new set in a single undo entry.

**Compatibility view (row 55).** `fc-geom` gained `source_shapes` and
`compare_shapes`; the preview now carries per-shape paths, points, start points,
centres and per-segment status. The view colours and numbers shapes by shape
order, draws start-point and shape-centre connectors, colours segments
green/yellow(>20°)/red, and repairs a shape shifter by dragging one shape's
handle onto another — `DesignSpaceCommand::ReorderShapes`, one undo entry,
reordering only the named source (or every source when no layer is named).

**Units per em and per-font editor settings (row 53).**
`variation.rescale_upm` changes the units per em and scales outlines, component
offsets, anchors, advances, vertical metrics, master metrics, zones, stems,
PostScript private values, guides and kerning — each group opt-in — as one
atomic `Command::Batch`. It refuses before touching the document when the source
carries data it does not model, and reports what it cleared or left stale.
`fontinfo.editor_settings_get`/`_set` store grid size, grid subdivision and the
three keyboard increments in the project and resolve them against the
application preferences.

**Scoped information (row 52).** A `Scopes` section in Font Info lists every
name, legal, identity and metric field with the scope that supplies its
effective value (Family → Font → Master → Export), per-source values, override
markers and the effective-scope column. Named instances gained typed export
overrides (`weight_class`, `width_class`, `postscript_font_name`,
`preferred_family_name`, `preferred_subfamily_name`) that `fc-compile`
applies _after_ the classes it derives from the instance location, so an
explicit value wins.

## Review fixes (second round)

The review returned **ACCEPT WITH FIXES** with nine findings; all nine are
fixed, each with a test.

| #   | Finding                                                                                        | Fix                                                                                                                                                                                                                                                             |
| --- | ---------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `scopedInfo.ts` marked any instance value as an override, so a repeated value looked like one. | An entry is emitted only where the value differs from what it inherits, and `effective: 'export'` only when some entry really differs. The same false positive in `AxesTable.tsx` and `InstanceEditor.tsx` is fixed through shared `overridesFamily`/`overridesStyle`. |
| 2   | Weight and width class read `—` although `instance_metrics.rs` derives them per instance.      | `widthClassOf` mirrors the compiler's piecewise map; each instance gets a derived entry at export scope, marked "derived from the instance location", and an explicit override replaces it without the marker.                                                    |
| 3   | Required metrics were hardcoded as overridden; optional metrics showed no font value.          | Required metrics compare against the master at every axis default; optional metrics show the shared automatic default in the Font column.                                                                                                                        |
| 4   | `rescale_upm` left metrics formulas unscaled (`=A+25` stayed while the units per em doubled).  | `scale_metrics_key` rewrites constants, automatic offsets, additive adjustments and `@height` measurements in both the glyph and the layer scope inside the same batch, leaves ratios alone, and refuses — before any change — a formula it cannot rewrite.        |
| 5   | The units-per-em help text claimed alternate layers must be baked first.                       | Corrected: alternate layers and metrics formulas scale with their group; the blockers are layer effects, reusable constructions, linked nodes, colour outlines and editable MATH or SVG.                                                                          |
| 6   | The axis sliders stepped by `(max - min) / 1000`, so no arrow key reached 500.                  | `axisStep` picks the coarsest step that still reaches the default, the maximum and every named location (1 for a weight axis, 0.5 for a width axis carrying 87.5); each axis gained a `NumberField` and a reset button.                                            |
| 7   | A profile's stored file name beat every matrix row, so a named profile collided on two rows.   | The row's name is appended to the stored stem; a configuration's own file name, being per-row already, wins.                                                                                                                                                     |
| 8   | Copy and naming details.                                                                       | The stored-configuration picker has its own key; the CLI test comment states the real precedence; the user doc calls the picker **Source to reorder**; name ID 16 is written whenever 17 is, and name 4 (and through it the CFF FullName) is rebuilt from the pair. |
| 9   | The acceptance script asserted "no composite remains" on a fixture with no composites.         | The script now patches a real component glyph into a converted project, proves it compiles to a composite, then proves decomposition removes it and keeps the offset outline.                                                                                     |

New tests: `metrics_key_tests` and `metrics_formulas_are_rescaled_in_both_scopes`
/ `a_formula_this_operation_cannot_rescale_refuses_before_any_change`
(`fc-api`), `a_named_profile_gives_every_matrix_row_its_own_file` (`fc-api`),
`a_preferred_subfamily_override_still_writes_its_family_name` (`fc-compile`),
`designCoordinates.test.ts` (5) and `VariationPreview.test.tsx` (4), plus the
extended `scopedInfo.test.ts` (13). `scripts/p5f_export_acceptance.py` now runs
**70** independent checks, all passing.

## Commands

All Rust commands ran under `flock .scratch/p5f/cargo.lock` with an absolute
task `TMPDIR`. Frontend commands used one Vitest worker.

```text
cargo test -p fc-compile --test designspace_subset  10 passed
cargo test -p fc-compile --test bracket_conditions   8 passed
cargo test -p fc-compile --test variable            15 passed
cargo test -p fc-api --lib                         238 passed
cargo test -p fc-api --test export_profiles          6 passed
cargo test -p fc-api --test variation_compatibility  8 passed
cargo test -p fc-api --test rescale_upm             10 passed
cargo test -p fc-api --test variation                2 passed
cargo test -p fc-api --test export                  16 passed
cargo test -p fc-api --test export_options           4 passed
cargo test -p fc-cli --test profiles                 8 passed
cargo test -p fc-cli --test cli                     14 passed
cargo test -p fc-doc --test designspace_commands    16 passed
cargo test -p fc-doc --test commands_undo            5 passed
cargo test -p fc-formats --test designspace_rules    3 passed
cargo test -p fc-formats --test glyphs              15 passed
cargo test -p fc-model --test serde_roundtrip        6 passed
cargo test -p fc-geom                              113 passed across 21 targets
```

```text
cargo clippy -p fc-model   --all-targets -- -D warnings   clean
cargo clippy -p fc-geom    --all-targets -- -D warnings   clean
cargo clippy -p fc-compile --all-targets -- -D warnings   clean
cargo clippy -p fc-doc     --all-targets -- -D warnings   clean
cargo clippy -p fc-formats --all-targets -- -D warnings   clean
cargo clippy -p fc-api     --all-targets -- -D warnings   clean
cargo clippy -p fc-cli     --all-targets -- -D warnings   clean
```

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 <22 owned files>
  Test Files  22 passed (22)
       Tests  204 passed (204)

pnpm --dir app exec eslint src/dialogs src/stores/exportProfileStore*.ts \
  src/views/fontinfo src/views/glyph/panels/{GlyphVariationPanel.tsx,BracketConditionFields.tsx,bracketConditions.ts}
  exit 0, no diagnostics

pnpm --dir app check
  no diagnostics

FC_API_PORT=8806 FC_UI_PORT=1506 pnpm --dir app exec playwright test export.spec.ts
  2 passed (35.4s)
```

`app/e2e/export.spec.ts` gained a spec that saves a profile through the dialog,
reads it back out of the document with `export.profiles_get`, reloads the page
and applies it again. The same commit fixes a pre-existing ambiguity in the
older spec in that file: `getByRole('tab', { name: 'TTF' })` also matched the
`CBDT (TTF)` and `sbix (TTF)` tabs, so both selectors now pass `exact: true`.

`pnpm --dir app check` and the full Vitest suite also cover other tasks' files;
at the time of this run they were clean. Prettier was applied only to the files
this task owns; `crates/fc-doc/src/fcf/schema.md` is outside the app's Prettier
scope and was already unformatted at HEAD, so it was left unreformatted.

## Independent verification

`scripts/p5f_export_acceptance.py` builds a two-axis, four-master designspace
with FontTools, drives the `fontcarver` command line for every format and
production option the checklist names, and reads each output back with
FontTools, FreeType and HarfBuzz — none of which share code with the exporter.

```text
python3 scripts/p5f_export_acceptance.py --cli target/debug/fontcarver --out <scratch>
70 passed, 0 failed
```

What it proves, by row:

- **54** A master subset keeps only the varying axis (`['wght']`) and drops the
  instances outside the retained space; a stored variable configuration's axis
  and instance subsets reach `fvar` (`['wght']`, `['Bold', 'Regular']`).
- **58, 63** The profiles are patched into the `.fcf` **as JSON, by an external
  tool**, and `fontcarver profiles` lists them, `fontcarver build --profile Web
--instances all --out-dir` writes one WOFF2 per active instance under the
  profile's subfolder, `--preflight` prints the report, and `--profile` with
  `--remove-overlap` exits 2 without writing.
- **59** Variable TTF (`fvar` + `gvar`), variable OTF (`CFF2`), static instance
  OTF (`CFF `, no `fvar`, `usWeightClass` 700), WOFF and WOFF2 flavours with the
  same glyph order, designspace + four UFO packages that build again, and a
  single UFO package. FreeType renders and HarfBuzz maps both characters for
  each compiled output.
- **58** A profile that stores its own file name still writes one file per
  matrix row (`Acceptance-Regular.ttf`, `-Bold`, `-Condensed`,
  `-Bold-Condensed`), and a variable configuration's own file name wins over it.
- **60** Extrema insertion raises the on-curve count (3 → 4); rounding makes
  every `glyf` coordinate integral; a component glyph patched into the project
  compiles to a real composite referencing `A`, and with
  `--decompose-components` no composite remains while the outline keeps the
  component's 40-unit offset; a production name replaces the source name in the
  glyph order while the plain build keeps it; subroutinization produces one
  global subroutine and a smaller CFF (976 ≤ 1088 bytes) that FreeType still
  renders.

`crates/fc-cli/tests/profiles.rs` additionally asserts that the same stored
profile produces **byte-identical** output from the command line and from an
in-process `session.export`, which is row 63's parity claim.

## Browser evidence

`fc-devserver` on 8806 with a real project, Vite on 1506, driven headlessly with
the `agent-browser` skill. The project was a `.fcf` converted from the
acceptance designspace, with export profiles, a variable-font configuration, a
compound alternate region and a shape-shifter glyph patched in as JSON.

| Row    | What was done in the browser                                                                                                                                                                                                                                                                                                                                                                                     | Screenshot                                                                                      |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| 58     | The dialog listed the project's `Web` and `Desktop` profiles; applying `Web` switched the format tab to WOFF2, the destination to `.woff2` and loaded its `web` subfolder. Saving a new profile reached `export.profiles_get`.                                                                                                                                                                                   | [export-profiles-project.png](screenshots/p5f-f-export/export-profiles-project.png)             |
| 58     | Preflight reported `1 error · 0 warnings · 1 note`, named the `duplicate-unicode` error on `B`, and its glyph button closed the dialog and opened `B` in an Edit tab.                                                                                                                                                                                                                                            | [export-preflight.png](screenshots/p5f-f-export/export-preflight.png)                           |
| 58, 54 | The matrix planned 6 destinations from 3 rows × 2 profiles and wrote all of them; the variable rows reported the dropped axis, the omitted instances and the trimmed conditions. FontTools confirmed `axes ['wght']`, `instances ['Bold','Regular']`.                                                                                                                                                            | [export-matrix-results.png](screenshots/p5f-f-export/export-matrix-results.png)                 |
| 55     | The two shapes of the shifter glyph drew in different colours with their shape indices, and the start-point connectors crossed (`0,0 600,0 …` / `600,0 0,0 …`). _Move shape 1 later_ straightened them in one undo entry; dragging one handle onto the other swapped them back.                                                                                                                                  | [compatibility-shape-order.png](screenshots/p5f-f-export/compatibility-shape-order.png)         |
| 56     | The region row read `[75‹wdth‹90, 500‹wght] 4 sources`; adding a `wght` upper bound and applying moved all four sources to `[75‹wdth‹90, 500‹wght‹700]`, and one undo restored all four.                                                                                                                                                                                                                         | [edit-view-conditional-sources.png](screenshots/p5f-f-export/edit-view-conditional-sources.png) |
| 57     | The Edit inspector's axis sliders scrubbed the interpolated advance live (600 → 660 at wght 700, → 620 at wdth 75) with the overlay on the editing canvas and no document mutation.                                                                                                                                                                                                                              | [edit-view-axis-sliders.png](screenshots/p5f-f-export/edit-view-axis-sliders.png)               |
| 52     | The Scopes table showed Family/Font/Master/Export columns with per-source values, override markers and the effective scope per field. Re-taken after the review fixes: the family name every instance repeats is **not** marked an override, only the three differing style names are, and the weight and width classes read 400/700/400/900 and 5/5/3/3 with "derived from the instance location" on every value the compiler derives and none on the explicit 900. | [font-info-scopes.png](screenshots/p5f-f-export/font-info-scopes.png)                           |
| 57     | Ten `Shift+↑` presses in the Weight field moved 400 → **500** exactly (slider and field in step) and the kernel returned `Advance: 620.0`, the correct interpolation of 600 and 660. The per-axis reset returned 400 and `Advance: 600.0`, and both reset buttons then disabled themselves.                                                                                                                       | [design-space-axis-fields.png](screenshots/p5f-f-export/design-space-axis-fields.png)           |
| 53     | The Settings section stored a per-font grid size of 2 (`Stored in this font: 2`) while subdivision and increments still read `Inherited from preferences`; changing units per em to 2000 with every group ticked reported `Scaled by 2.0000: 6 glyphs, 28 layers` and the stale-table warning. The API confirmed ascender 800 → 1600, advance 600 → 1200, point x 100 → 200 and bracket bounds unchanged at 500. | [font-info-settings-upm.png](screenshots/p5f-f-export/font-info-settings-upm.png)               |

The browser run found one real defect that the Rust tests had not: an axis
subset left alternate-layer condition ranges pointing at the removed axis, so a
variable build of that configuration failed with `invalid alternate layer
condition`. The fix (trim the ranges, drop an alternate that keeps none, report
both) and its two regression tests are in `fc-compile`.

## Per-row status

| Row | Status                      | Notes                                                                                                                                                                                                                                                                                                               |
| --- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 52  | **Complete within a limit** | Four-scope breadcrumb with override markers, revert actions and export overrides that reach the compiled font. FontCarver is one document per font, so the family scope is the family-wide field set inside the document, not a shared parent file. Recorded, not claimed as more.                                  |
| 53  | **Complete within a limit** | Per-master metrics/zones/stems already existed; this task adds the units-per-em change with scaling and the per-font grid, subdivision and keyboard increments with inheritance. The editor's own _use_ of those per-font values is one patch in task c's `GlyphCanvas.tsx`/`editorStore` — see "What I need".      |
| 54  | **Complete**                | Axes, masters, instances, named variable-font configurations with axis/instance/master subsetting, selectable in the dialog, the matrix and the CLI, verified with FontTools.                                                                                                                                       |
| 55  | **Complete within a limit** | Shape colours, indices, start-point and centre connectors, green/yellow/red segments and drag-to-swap with a keyboard equivalent. Anchor connectors are not drawn, and FontLab-style node-insertion repair is out of scope.                                                                                         |
| 56  | **Complete within a limit** | Compound condition sets end to end: model, validation, interpolation, expansion, `rvrn`, designspace rules, Glyphs `axisRules`, and a region editor. A set is a conjunction of per-axis ranges; a disjunction is expressed as another alternate layer. A layer that is both brace and bracket is still unsupported. |
| 57  | **Complete**                | Sliders in the Edit inspector scrub an interpolated overlay on the canvas against the real kernel, each with an axis-unit step, a number field and a per-axis reset.                                                                                                                                                |
| 58  | **Complete**                | Profiles in the project file, an instance × profile matrix that also takes variable-font configurations as rows, and a preflight report with click-to-jump — all with real-kernel browser evidence.                                                                                                                 |
| 59  | **Complete**                | OTF (CFF), TTF, WOFF, WOFF2, variable TTF and UFO + designspace re-verified through the current CLI with FontTools, FreeType and HarfBuzz.                                                                                                                                                                          |
| 60  | **Complete**                | Overlap removal, extrema insertion, coordinate rounding, decomposition, production names and CFF subroutinization, each verified on the emitted font.                                                                                                                                                               |
| 63  | **Complete**                | `fontcarver profiles`, `build --profile/--variable-config/--instances/--out-dir/--preflight`, with byte-identical output to an in-process export of the same profile.                                                                                                                                               |

## Remaining limits

- **Family scope.** There is no cross-document family container; the family
  scope is the family-wide field set inside one document.
- **Per-font editing values.** The kernel stores and resolves grid size, grid
  subdivision and keyboard increments, and Font Info edits them. The glyph
  editor still reads the application preference; wiring it is one change in
  task c's files.
- **Alternate layers.** Condition sets are conjunctions of per-axis ranges. A
  layer cannot be both intermediate and conditional. Glyphs' per-configuration
  `Remove Glyphs` / `Add Feature` parameters are not modelled.
- **Units per em.** The change refuses, rather than guessing, when a layer
  carries effects, reusable constructions, linked nodes, guide-owned nodes or
  colour outlines, or when the font has editable MATH or SVG data. Preserved
  imported tables still describe the previous units per em and are reported.
  Metrics formulas are rewritten in both scopes; a formula outside the grammar
  `fc-doc` parses is refused with the glyph and the formula named, because
  `crates/fc-doc/src/metrics.rs` belongs to another task and could not gain a
  shared scaling helper in this round. Moving `scale_metrics_key` there, beside
  `parse_key`, is the obvious follow-up.
- **Compatibility view.** Anchors are not connected across sources, and repair
  never inserts or deletes points.
- **Profiles.** A profile is per document. There is no user-global profile
  library, and no "test install" step.

## Shared and unowned files touched

`crates/fc-api/src/api.rs` (three `METHODS` rows and three dispatch arms, plus
`variation.rescale_upm` and the two editor-settings methods),
`crates/fc-api/src/bin/gen_ts.rs` (type registrations and one import repair),
`crates/fc-model/src/{layer.rs,lib.rs,font.rs,query/brackets.rs}`,
`crates/fc-doc/src/command/{designspace.rs,apply.rs}`,
`crates/fc-formats/src/{ufo/conditional.rs,ufo/designspace.rs,glyphs/conditional.rs}`,
`crates/fc-doc/src/fcf/schema.md`, `app/src/i18n/en.json`, and the literal
sites in `crates/fc-model/tests/serde_roundtrip.rs`,
`crates/fc-doc/tests/designspace_commands.rs`,
`crates/fc-formats/tests/designspace_rules.rs`. Every one is an additive patch
made immediately after re-reading the file. The review round added two more:
`app/src/i18n/en.json` (one corrected string, two new keys) and
`docs/user/variable-and-cjk.md`.

## Housekeeping

`python3 scripts/check_artifacts.py` reported PASS at task start, between build
batches and at the end:

```text
build: 9.40 GiB, 18,169 files
temporary: 0.00 GiB, 3 files
workspace free: 629.43 GiB
PASS
```

The review round ran the same way: `fc-devserver` on 8806 and Vite on 1506 were
started for the browser evidence and stopped again, only this task's own
`agent-browser` session was closed, and the scratch directory was removed. Note
for whoever runs the next browser round in this checkout: headless Chrome is
killed by the shell sandbox (SIGTRAP, no output) and needs the sandbox off.

The build cache is shared with seven other agents and reached 19.3 GiB against
the 20 GiB ceiling partway through this task; no cache was cleaned here, because
the orchestrator reserved that, and it was back under the 10 GiB warning by the
end. The development server, Vite and the browser session this task started were
stopped, only this task's own `agent-browser` session was closed, and
`.scratch/p5f-f-export/` was removed. The retained evidence is
`scripts/p5f_export_acceptance.py`, this report and the nine screenshots under
`docs/screenshots/p5f-f-export/`.
