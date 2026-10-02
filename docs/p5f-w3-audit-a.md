# Wave 3 independent audit: rows 1–56

Date: 2026-10-02. Reviewed the integrated source and the working-tree metrics
move/wording follow-ups starting from `7aaace3`; concurrent wave-3 changes are
not assumed accepted. This is a source-and-evidence review, not a fresh runtime
acceptance run. No application code was changed and no broad build was run.

`Supported` below means the current implementation agrees with the specific
bounded claim and its historical acceptance evidence. It does not mean every
listed test was rerun here. `Gap` identifies an ordinary missing or incorrect
workflow; `Qualify` identifies a claim requiring narrower wording. Parent-owned
integration testing and repair evidence must supersede gaps before closure.

## Findings requiring action

1. **Row 19: sequential vertical-bearing edits can use stale values.**
   [VerticalMetricsPanel](../app/src/views/glyph/panels/VerticalMetricsPanel.tsx)
   stores pending origin/advance overrides in `sent`, but its BSB handler
   calculates the new advance using render-time `origin`. With stored origin
   880, pending origin 910, ink minimum −100 and requested BSB 250, it sends
   advance 1230/origin 910 instead of advance 1260/origin 910. The resulting BSB
   is 220. The panel also retains a rejected pending override: entering BSB
   70000 and then a valid TSB can resend the invalid advance. The existing
   [tests](../app/src/views/glyph/panels/VerticalMetricsPanel.test.tsx) cover
   advance→origin/TSB and resetting the rejected field's display, but neither
   origin→BSB nor a valid edit following rejection. Reported to the parent for
   a separate repair and regression tests.
2. **Row 39: selected-side ×100 and exact-one keyboard modes were absent at
   the reviewed baseline.** [metricShortcut](../app/src/views/text/textMetrics.ts)
   implements configured-step brackets and Shift×10, refuses Mod+brackets for
   navigation, and has only a macOS-specific 100-unit legacy arrow form. A
   numeric field does not satisfy an explicit shortcut requirement. The parent
   has a separate task for this gap.
3. **Row 35: Fix All is deliberately a partial pass.**
   [outlineAudit](../app/src/views/glyph/outlineAudit.ts) selects one structural
   kind per contour and one extremum per segment; a second press may be needed.
   This is documented and avoids stale indexes, but remains ordinary workflow
   completeness rather than a research-grade boundary. Retain an explicit
   qualifier, or rederive successive repairs in one atomic undoable operation.
4. **Row 46: distinguish the typed library from the running editor.**
   [fea_ir](../crates/fc-layout/src/fea_ir/mod.rs) has substantive typed rules,
   preservation and round-trip tests. Repository-wide call-site inspection
   found `parse_fea` used only by its tests, not by production API/compile/UI
   callers. This supports the library capability; it does not prove the visual
   editor uses that IR. The parser rejects `include(...)` with an actionable
   error; preserving imported source text is a separate claim.
5. **Stale statements:** row 53's export acceptance still says editor wiring
   needs another task, although `GlyphView` now consumes
   `useFontEditorSettings`; row 20 says 16 strip entries although the registry
   now has 17 including Magnet. Row 10 uses virtualized Canvas2D; hardware GPU
   acceleration is browser-dependent and is not established by a frame budget.
   Native macOS/Windows execution remains unproved (row 9).

## Per-row source cross-check

The historical evidence links below are [shell](p5f-a-shell-acceptance.md),
[inventory](p5f-b-inventory-acceptance.md), [editor](p5f-c-editor-acceptance.md),
[metrics](p5f-d-metrics-acceptance.md), [features](p5f-e-features-acceptance.md),
[export/design space](p5f-f-export-acceptance.md) and
[CJK](p5f-l-cjk-acceptance.md). Their stated limits remain part of each claim.

| Row | Result                                        | Independent source/evidence check                                                                                                                                                                                                                                                                                                                                                                            |
| --- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | Supported on Linux                            | [windowLifecycle](../app/src/shell/windowLifecycle.ts) routes a newly opened document to its own native window; [windows.rs](../src-tauri/src/windows.rs) validates labels/targets. [workspaceTabCommands](../app/src/shell/workspaceTabCommands.ts) maps pinned Overview, ordinals and cycling. Shell acceptance distinguishes browser popups from native parity.                                           |
| 2   | Supported, platform qualification             | [windowActions](../app/src/shell/windowActions.ts) records detachment/rejoin; native destruction emits the close event in [windows.rs](../src-tauri/src/windows.rs). Shell review includes detached Inspector data loading and close restoration; actual macOS/Windows execution is still separate.                                                                                                          |
| 3   | Supported                                     | [AppShell](../app/src/shell/AppShell.tsx) docks contextual sides; [EditInspector](../app/src/views/glyph/panels/EditInspector.tsx) composes real section controls. Shell/component evidence covers remembered collapse state and contextual content.                                                                                                                                                         |
| 4   | Supported                                     | [PreviewStrip](../app/src/views/text/PreviewStrip.tsx) has pointer capture, keyboard resize, min/max/value ARIA and persisted height; [tests](../app/src/views/text/PreviewStrip.test.tsx) exercise resize and detachment. Collapse is an enabled-only command.                                                                                                                                              |
| 5   | Supported                                     | [usePaletteEntries](../app/src/shell/usePaletteEntries.ts) merges commands, inventory/saved filters, scripts and debounced glyph results. Shell review tests displayed-order navigation, so highlighted and executed entries agree.                                                                                                                                                                          |
| 6   | Supported, native-project scope               | [workspaceProject](../app/src/stores/workspaceProject.ts) hydrates and persists named presets through session methods; [workspaceLifecycle](../app/src/shell/workspaceLifecycle.ts) restores per-document layouts and detached views. Unsaved/non-native files explicitly use machine-local storage.                                                                                                         |
| 7   | Supported, recovery-location qualification    | [project.rs](../crates/fc-doc/src/fcf/project.rs) publishes per-glyph revisions before atomic manifest replacement, validates paths, and prunes only previously referenced files. [directory_project tests](../crates/fc-doc/tests/directory_project.rs) cover unchanged resaves, interrupted writes and symlinks. Recovery points back from a separate recovery directory, not an adjacent project sidecar. |
| 8   | Supported                                     | [HistoryPanel](../app/src/shell/HistoryPanel.tsx) renders scopes/current/undone entries and submits expected history length; [doc handler](../crates/fc-api/src/handlers/doc.rs) validates and rolls back a refused replay. Shell acceptance links stale/read-only and gesture-scope cases.                                                                                                                  |
| 9   | Partial                                       | [shellPlatformParity tests](../app/src/shell/shellPlatformParity.test.ts) prove one definition and simulated Mod dispatch, not native key events. No reviewed macOS/Windows runtime evidence closes parity.                                                                                                                                                                                                  |
| 10  | Supported, qualify renderer/sort              | [GlyphGrid](../app/src/views/font/GlyphGrid.tsx) virtualizes visible cells plus overscan; [GlyphCell](../app/src/views/font/GlyphCell.tsx) draws Canvas2D. [GlyphTableHeader](../app/src/views/font/GlyphTableHeader.tsx) has drag/keyboard column moves and ascending sort. Inventory performance evidence does not independently prove GPU acceleration or descending sorting.                             |
| 11  | Supported                                     | [glyphs handler](../crates/fc-api/src/handlers/glyphs.rs) computes each badge from glyph status, annotation/custom metadata, compatibility, export and lock data; [GlyphCell](../app/src/views/font/GlyphCell.tsx) exposes translated accessible status. Inventory filters share those sources.                                                                                                              |
| 12  | Supported                                     | [colorLabels](../app/src/views/font/colorLabels.ts) contains all 12 stable labels; the glyph handler reads an independent layer label. [glyphTags](../app/src/views/glyph/panels/glyphTags.ts) implements completion and deduplication; inventory evidence covers batched undo.                                                                                                                              |
| 13  | Supported                                     | [GlyphSearch](../app/src/views/font/GlyphSearch.tsx) offers all four scopes, case and regex controls, debounce and inline invalid state. Inventory evidence includes kernel validation and unchanged document on invalid patterns.                                                                                                                                                                           |
| 14  | Supported, bounded repertoire display         | [inventory coverage](../crates/fc-api/src/handlers/inventory.rs) counts assigned scalars and produces missing-member recipes; [SidebarFilters](../app/src/views/font/SidebarFilters.tsx) connects badges and generation. Repertoire sections are restricted to those the document already touches.                                                                                                           |
| 15  | Supported                                     | [smartFilterBuilder](../app/src/views/font/smartFilterBuilder.ts) consumes the generated kernel vocabulary and serializes a predicate tree to DSL. [smart_filters tests](../crates/fc-api/tests/smart_filters.rs) and inventory evidence cover project persistence, parsing and builder parity.                                                                                                              |
| 16  | Supported, stated overlay boundaries          | [database](../crates/fc-glyphdata/src/database.rs) embeds the versioned JSON and validates project overrides for names, scalars, anchors and composition. Inventory output evidence checks applied metadata/composition independently; production naming is an export option.                                                                                                                                |
| 17  | Supported                                     | [addGlyphs](../app/src/dialogs/addGlyphs.ts) delegates preview/apply recipes to the kernel; [batchRename](../app/src/dialogs/batchRename.ts) implements literal/regex replacement. [batch_rename tests](../crates/fc-doc/tests/batch_rename.rs) cover transactional reference changes, with browser acceptance for preview/apply.                                                                            |
| 18  | Supported, system-font dependent              | [cellRenderer](../app/src/views/font/cellRenderer.ts) paints empty encoded glyphs in a muted system-font fallback without creating outlines; [CjkFilters](../app/src/views/font/CjkFilters.tsx) exposes radicals/strokes/blocks. Visibility depends on an installed face covering the scalar.                                                                                                                |
| 19  | Gap at reviewed baseline                      | [VerticalMetricsPanel](../app/src/views/glyph/panels/VerticalMetricsPanel.tsx) has all four controls and real-kernel/export evidence, but sequential origin→BSB edits and recovery after rejected values are incorrect as described above. [vertical tests](../crates/fc-doc/tests/vertical.rs) prove model guards/persistence, not those UI sequences.                                                      |
| 20  | Supported                                     | [tool registry](../app/src/editor/tools/registry.ts) includes every requested tool plus Magnet; Text and Pixel intentionally hand off to their own editing surfaces. Editor acceptance covers strip, shortcut and palette routes; current count is 17.                                                                                                                                                       |
| 21  | Supported                                     | [registry](../app/src/editor/tools/registry.ts) defines the 200 ms hold threshold and stable key table; shell/tool-shortcut evidence covers tap/borrow/release and keymap overrides.                                                                                                                                                                                                                         |
| 22  | Supported                                     | [editorController](../app/src/editor/editorController.ts) routes smooth-node actions to command handling, with kernel [smooth.rs](../crates/fc-doc/src/command/outline/smooth.rs). Editor acceptance checks double-click/Return and Alt preservation.                                                                                                                                                        |
| 23  | Supported, cubic G2 scope                     | [smoothCommands](../app/src/views/glyph/smoothCommands.ts) offers persistent G2 and one-shot Harmonize only for valid cubic joins; [harmonize](../app/src/editor/harmonize.ts) rejects unsupported/nonconvergent previews. [super_smooth tests](../crates/fc-doc/tests/super_smooth.rs) cover model persistence/constraints. General quadratic G2 remains outside the claim.                                 |
| 24  | Supported, native-input qualification         | [modifiers](../app/src/editor/modifiers.ts) separates physical Control from platform Mod; [nudge](../app/src/editor/gestures/nudge.ts) selects configured 1/10/100 steps; [editorSnapping](../app/src/editor/editorSnapping.ts) suspends snap with Control. Native parity is not proved by simulated events.                                                                                                 |
| 25  | Supported                                     | [drag tests](../app/src/editor/gestures/drag.test.ts) explicitly exercise proportional adjustment of adjacent handles; the controller also applies the same modifier grammar to nudge runs. Editor acceptance covers the ordinary gesture.                                                                                                                                                                   |
| 26  | Supported                                     | [editorActions](../app/src/editor/editorActions.ts) maps Shift-segment insertion to extremum/inflection candidates, with controller regression for nearest extremum; [extrema tests](../crates/fc-geom/tests/extrema.rs) distinguish normal and forced insertion.                                                                                                                                            |
| 27  | Supported                                     | [editor acceptance](p5f-c-editor-acceptance.md) is backed by actual path commands and [corner geometry tests](../crates/fc-geom/tests/corner.rs) for Open Corner/Reconnect. All-master direction/tidy paths and inverse history are retained.                                                                                                                                                                |
| 28  | Supported                                     | [InfoBox](../app/src/views/glyph/InfoBox.tsx) selects object/multi-point fields; [InfoMultiRows](../app/src/views/glyph/InfoMultiRows.tsx) uses one transform about a nine-point origin with proportion lock. [NumberField](../app/src/components/NumberField.tsx) validates, commits and steps; controls are keyboard reachable.                                                                            |
| 29  | Supported                                     | [hud](../app/src/editor/hud.ts) formats delta, polar length/angle, stem and snapped target; controller gesture state feeds the cursor-following display. Editor evidence distinguishes the measured fields rather than a static mockup.                                                                                                                                                                      |
| 30  | Supported, measurement markers are session UI | [GuideRows](../app/src/views/glyph/panels/GuideRows.tsx) edits local/master-global named guides and locks; [GuideAnchorRow](../app/src/views/glyph/GuideAnchorRow.tsx) rebases offsets when anchoring changes. Measurement toggles are owner-scoped session state; guide geometry persists.                                                                                                                  |
| 31  | Supported                                     | [snapping](../app/src/editor/snapping.ts) handles target families and distance resolution; preference/store/controller wiring reaches runtime snapping. Editor acceptance pins ghost component geometry and independently configurable distances.                                                                                                                                                            |
| 32  | Supported                                     | [layerCommands](../app/src/views/glyph/layerCommands.ts) supplies edit/create, selection transfer, copy, swap and clear for the current master; [LayersPanel](../app/src/views/glyph/panels/LayersPanel.tsx) exposes the same operations and visibility.                                                                                                                                                     |
| 33  | Supported, bounded cascade                    | [component_locks](../crates/fc-doc/src/command/component_locks.rs) rejects destructive lock bypasses and aligns unlocked references. [tests](../crates/fc-doc/tests/component_locks.rs) cover chained attachment and dependent realignment in the same undo step. Automatic propagation remains bounded to four rounds.                                                                                      |
| 34  | Supported                                     | [useMarkCloud](../app/src/views/glyph/panels/useMarkCloud.ts) pages real Mark glyphs with bounded detail concurrency and stale-response guards; [MarkCloudPreview tests](../app/src/views/glyph/panels/MarkCloudPreview.test.tsx) cover anchor-based placement and UI states.                                                                                                                                |
| 35  | Qualify / incomplete all-at-once repair       | [outlineAudit](../app/src/views/glyph/outlineAudit.ts) computes four real checks and derives repairs from source indexes. The live panel has a 250 ms refresh ceiling, but Fix All intentionally leaves other repair kinds/extrema for later passes; [tests](../app/src/views/glyph/outlineAudit.test.ts) explicitly encode this limit.                                                                      |
| 36  | Supported                                     | [LayersPanel](../app/src/views/glyph/panels/LayersPanel.tsx) exposes add/copy/rename/remove and master/background/source roles through commands; editor acceptance covers native persistence and background workflows.                                                                                                                                                                                       |
| 37  | Supported, horizontal formula scope           | [metrics resolver](../crates/fc-doc/src/metrics.rs) parses constants, references, mirror, local, height and automatic offsets, validates finite values, prevents cycles and measures actual geometry. Vertical formula fields are deliberately refused; vertical metrics have their own explicit controls.                                                                                                   |
| 38  | Supported                                     | [metricsCommands](../app/src/views/glyph/panels/metricsCommands.ts) updates an edited glyph or selected batch for current/all masters; badges use kernel metric status. Missing current master is refused rather than silently updating all.                                                                                                                                                                 |
| 39  | Gap at reviewed baseline                      | [textMetrics](../app/src/views/text/textMetrics.ts) has coalesced undoable LSB/RSB/both edits but incomplete portable ×100/exact-one shortcut modes. Numeric controls remain functional; separate wave-3 repair must provide final evidence.                                                                                                                                                                 |
| 40  | Supported                                     | [kerningModes](../app/src/views/text/kerningModes.ts) disables kern/vkrn when off and provides command-time spacing guards when locked. Metrics commands and vertical panel consult the guard; metrics acceptance includes locked-mode tests.                                                                                                                                                                |
| 41  | Supported                                     | [useTextKerning](../app/src/views/text/useTextKerning.ts) resolves selected pairs and modifier-held class suppression; model group membership and exception precedence are covered by metrics/kernel evidence.                                                                                                                                                                                               |
| 42  | Supported                                     | [drawTextGaps](../app/src/views/text/drawTextGaps.ts) colors every applied gap by sign and rings exceptions; [useKerningGhosts](../app/src/views/text/useKerningGhosts.ts) loads group outlines with owner/master identity and stale-response protection.                                                                                                                                                    |
| 43  | Supported                                     | [KerningTable](../app/src/views/text/KerningTable.tsx) implements ascending/descending sorting, filtering, group rename, generation-checked maintenance and master-specific copy/paste policies. Metrics review/evidence covers scope and undo.                                                                                                                                                              |
| 44  | Supported                                     | [useSpacingMatrix](../app/src/views/text/useSpacingMatrix.ts) queries real metric/formula/beam data with complete identity checks; [SpacingMatrix](../app/src/views/text/SpacingMatrix.tsx) edits it. Metrics/editor acceptance records beam dragging, cut-off and saved proof strings; beam measurement shares kernel formula geometry.                                                                     |
| 45  | Supported, row 19 repair also applies         | Vertical kerning authoring is independent of horizontal data and compiled as vkrn; [vertical tests](../crates/fc-doc/tests/vertical.rs) cover commands/groups/undo. Metrics tables have independent CJK output evidence. The sequential Inspector defect in row 19 must also be repaired before calling every vertical-metrics workflow complete.                                                            |
| 46  | Qualify as library capability                 | [fea_ir](../crates/fc-layout/src/fea_ir/mod.rs) represents typed GSUB/GPOS syntax and preserved unknown statements; [round-trip tests](../crates/fc-layout/tests/fea_ir.rs) include compiled binary decompilation and malformed/include rejection. No reviewed production caller uses it.                                                                                                                    |
| 47  | Supported, applicable generators only         | [FeatureSourceTree](../app/src/views/features/FeatureSourceTree.tsx) shows prefix/class/feature/generated entries; [FeatureAutomaticSwitch](../app/src/views/features/FeatureAutomaticSwitch.tsx) freezes one entry or confirms replacement. Unsupported prefixes and mapping-based recipes give a disabled reason rather than pretending automatic inference exists.                                        |
| 48  | Supported, stated generation limits           | [generators](../crates/fc-model/src/features/generators.rs) contains the enumerated feature families; features acceptance verifies actual compiled shaping. locl/dlig use explicit mappings, compound figure suffixes are not inferred, and ccmp derives supported composition/anchors rather than arbitrary outlines.                                                                                       |
| 49  | Supported                                     | [model_layout_writers tests](../crates/fc-layout/tests/model_layout_writers.rs) exercise actual kern/mark/mkmk/curs output and shaping; features acceptance records source-vs-generated precedence rather than mere feature-tag presence.                                                                                                                                                                    |
| 50  | Supported                                     | [FeatureSourceEditor](../app/src/views/features/FeatureSourceEditor.tsx) wires diagnostic ranges, completion sources and class preview to real draft sessions; [FeatureClassPreview](../app/src/views/features/FeatureClassPreview.tsx) resolves names/outlines, reports incomplete lists and bounds its preview at 64 members.                                                                              |
| 51  | Supported                                     | Generated vert/vrt2 families and alternate handling are included in the model/compile dispatch; features acceptance checks feature-on/off vertical shaping, not merely emitted source.                                                                                                                                                                                                                       |
| 52  | Supported, one-document family scope          | [scopedInfo](../app/src/views/fontinfo/scopedInfo.ts) computes family/font/master/export origin and flags only differences from inheritance, including derived weight/width. Export acceptance verifies emitted overrides. No separate shared family parent file is implied.                                                                                                                                 |
| 53  | Supported; stale handoff note closed          | [variation handler](../crates/fc-api/src/handlers/variation.rs) rescales font-unit data atomically and validates per-font settings; [rescale tests](../crates/fc-api/tests/rescale_upm.rs) cover stale/invalid rejection, formula scaling, undo and reopen. [GlyphView](../app/src/views/glyph/GlyphView.tsx) consumes font settings and passes resolved snap/nudge values to the canvas.                    |
| 54  | Supported                                     | [DesignSpaceEditor](../app/src/views/fontinfo/DesignSpaceEditor.tsx) edits axes/masters/instances; [designspace_subset tests](../crates/fc-compile/tests/designspace_subset.rs) validate named configuration subsetting and real exports. Export acceptance includes independent FontTools inspection.                                                                                                       |
| 55  | Supported, stated repair scope                | [VariationCompatibility](../app/src/views/fontinfo/VariationCompatibility.tsx) draws shape indices, start/center connectors, segment status and drag/keyboard swaps. [API tests](../crates/fc-api/tests/variation_compatibility.rs) validate permutations, incompatible shapes and one undo entry. Anchor connectors and automatic node insertion are outside this accepted interaction.                     |
| 56  | Supported, conjunction regions                | [SourceLayerEditor](../app/src/views/glyph/panels/SourceLayerEditor.tsx) edits brace coordinates and per-axis bracket ranges; [bracket_conditions tests](../crates/fc-compile/tests/bracket_conditions.rs) validate compound regions and compiled variation conditions. OR is represented by another alternate layer; combined brace-and-bracket layers are unsupported.                                     |

## Initial follow-up diff review

- **Metrics helper move:** source body and its two test cases are unchanged;
  only module location, public visibility, resolver-local documentation and API
  call target changed. Safe to commit within
  [its recorded validation](p5f-metrics-key-move-acceptance.md).
- **Wording:** reviewed all English hunks, the Chinese delta, removed unused
  bare `cid.unmapped`, and changed PreviewStrip/WorkspaceDialog expectations.
  The counted plural keys remain; visible terminology agrees with controls.
  Safe to commit within [wording acceptance](p5f-wording-translation-acceptance.md).
- No new dependencies, translations, FontForge adaptations or shared code
  changes were introduced by this audit.

## Verification and resources

Read source, regression tests and acceptance records using `rg`, `sed`, `cat`
and `git diff`; extracted rows 1–56 with a small read-only Python script.
`rg -n 'parse_fea\(|write_fea\(|fea_ir::' crates app/src --glob
'!**/generated/**'` showed library definitions and tests only. No tests were
newly executed; parent owns the integrated gate and focused defect reproduction.

Initial `python3 scripts/check_artifacts.py`:

```text
build: 14.91 GiB, 18,131 files
temporary: 0.00 GiB, 4 files
workspace free: 607.04 GiB
warnings: build artifacts exceed 10 GiB; plan profile-cache cleanup
PASS
```

The documentation check passed: 56 unique sequential row IDs and every local
evidence link exists. Final artifact check (other tasks are active):

```text
build: 15.59 GiB, 19,162 files
temporary: 0.32 GiB, 2,579 files
workspace free: 606.07 GiB
warnings: build artifacts exceed 10 GiB; plan profile-cache cleanup
PASS
```

This task created no scratch directory and started no persistent process.
