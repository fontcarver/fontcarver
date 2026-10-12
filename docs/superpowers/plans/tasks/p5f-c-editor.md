# p5f-c: Outline editor tools, interaction and glyph panels

Area id: `c-editor`. Ports: API 8803, UI 1503. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                                                                                    | State at start                                                                                                                                                                                                                 |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 20  | Tools: Select, Lasso, Text, Pen, Pencil, Primitives (rect/ellipse), Knife, Eraser, Rotate, Scale, Measure, Zoom, Hand, Annotation, Pixel                       | All tool files exist under `app/src/editor/tools/`; Lasso/Eraser/Annotation integration needs acceptance. Pixel tool: verify it is reachable from the palette and hands over to the strike editor (task g owns its behaviour). |
| 21  | Tap-to-switch / hold-to-borrow tool activation                                                                                                                 | Reusable evidence (P1/T6); re-verify.                                                                                                                                                                                          |
| 22  | Smooth vs corner nodes with double-click / Return toggle; Alt variant preserving the outline                                                                   | Reusable evidence; re-verify.                                                                                                                                                                                                  |
| 23  | G2 "super-smooth" nodes + one-shot Harmonize                                                                                                                   | `harmonize.ts`, `fc-geom/harmonize.rs`; G2 node type and UI need acceptance.                                                                                                                                                   |
| 24  | Full modifier grammar: Shift constrain/×10, Cmd ×100, Alt alternate behaviour, Ctrl suspend snap                                                               | `modifiers.ts`; Ctrl snap suspension and platform combinations to verify.                                                                                                                                                      |
| 25  | Nudge (proportional handle adjustment) while dragging nodes                                                                                                    | Reusable evidence; re-verify.                                                                                                                                                                                                  |
| 26  | Shift-click a segment inserts a node at the nearest extremum/inflection; Add Extremes / Force Extremes                                                         | Segment insertion and Add Extremes exist; Force Extremes to verify/implement.                                                                                                                                                  |
| 27  | Tidy up Paths, Correct Path Direction (+ all-masters variants), Reverse Contours, Open Corner / Reconnect Nodes                                                | `pathOps.ts`, `pathCommands.ts`; all-masters variants, Open Corner, Reconnect to verify.                                                                                                                                       |
| 28  | Info box with editable X/Y/W/H, transformation origin, proportional lock, Tab traversal, arrow increments                                                      | Reusable evidence; re-verify.                                                                                                                                                                                                  |
| 29  | Cursor-following HUD: delta, angle, length, stem width                                                                                                         | Reusable evidence; re-verify.                                                                                                                                                                                                  |
| 30  | Local and global guides, angle lock, naming, LSB/RSB anchoring, measurement guides                                                                             | Guides exist; LSB/RSB anchoring to verify.                                                                                                                                                                                     |
| 31  | Snapping to nodes, handles, component interiors (ghost nodes), metrics, zones, guides, grid, background; each independently toggleable, configurable distances | `snapping.ts`, `editorSnapping.ts`; per-target toggles, zones/background and distances to verify.                                                                                                                              |
| 32  | Background layer: show, edit (Mod+B), selection→background, swap, clear, per-master                                                                            | Exists; shortcut and selection-only transfer to verify.                                                                                                                                                                        |
| 33  | Components with automatic alignment via anchors; decompose; lock component                                                                                     | `component_locks.rs`, `ComponentsPanel`; lock to verify.                                                                                                                                                                       |
| 34  | Anchors with `_`-prefixed mark anchors and a mark-cloud preview                                                                                                | `MarkCloudPreview`; see `docs/p5-34-mark-cloud-acceptance.md`.                                                                                                                                                                 |
| 35  | Always-on outline audit with inline markers, hover detail, Fix / Fix All, per-test toggles                                                                     | `OutlineAudit`, `outlineAudit.ts`; see `docs/p5-outline-audit-acceptance.md`.                                                                                                                                                  |
| 36  | Layers panel: masters, backups, background, per-glyph layer add/duplicate/rename                                                                               | `LayersPanel`, `layerWorkflows`; see `docs/p5-layer-command-workflows-acceptance.md`.                                                                                                                                          |
| 81  | High-DPI canvas, logical-pixel hit testing, configurable handle size                                                                                           | DPR rendering exists; configurable handle size to verify/implement.                                                                                                                                                            |
| 89  | Path focus and path/glyph locking                                                                                                                              | `editorMetadata*`; see `docs/p5-editor-metadata-acceptance.md`.                                                                                                                                                                |
| 90  | Measurement tool with rulers, intersection distances, italic-angle-aware deltas, drop-a-guide-mid-drag                                                         | `measure.ts`, `rulerGuides.ts`, `guideMeasure.ts`; italic deltas and guide drop to verify.                                                                                                                                     |
| 91  | Annotation tool (text, arrow, circle, plus, minus) with per-layer storage                                                                                      | `annotation.ts`, `drawAnnotations.ts`; verify all five kinds and persistence.                                                                                                                                                  |

Existing records: `docs/p5-outline-editor-acceptance.md`,
`docs/p5-outline-node-semantics-acceptance.md`, `docs/p5-path-operations-acceptance.md`,
`docs/p5-outline-audit-acceptance.md`, `docs/p5-editor-metadata-acceptance.md`,
`docs/p5-34-mark-cloud-acceptance.md`, `docs/p5-layer-command-workflows-acceptance.md`,
`docs/p5-tool-keymaps-acceptance.md`, `docs/p1-acceptance.md`,
`docs/superpowers/plans/tasks/p5-outline-*.md`, `p5-89-editor-metadata*.md`,
`p5-component-locking.md`, `docs/design/editor-ui.md`.

## Scope notes

- Every tool in row 20 must be selectable from the palette, by its shortcut
  and by the palette command; Lasso selects nodes by freehand region; Eraser
  removes nodes/segments under the pointer; Annotation creates the five kinds.
- Row 23: a G2 node type stored in the model (schema update), drawn
  distinctly, maintained on handle drag; Harmonize command on selection.
- Row 24: verify the exact grammar on Linux (Ctrl as Mod) and make the
  platform mapping explicit so macOS uses Cmd; Ctrl (or the documented
  equivalent) suspends snapping during a drag.
- Row 26: Force Extremes inserts extrema even when nearly present, per
  selection or whole glyph, with undo.
- Row 27: all-masters variants apply the same operation across compatible
  masters in one undo unit; Open Corner opens the selected corner into an
  overlapping construction; Reconnect Nodes rejoins two open ends.
- Row 31: preferences hold one toggle and one distance per snap target;
  component interiors expose ghost nodes when snapping.
- Row 35: audit runs live while editing (debounced), markers drawn inline,
  hover shows detail, Fix and Fix All apply the auto-fix in one undo unit,
  per-test toggles persist.
- Row 81: handle size preference (small/medium/large or px) applied to drawing
  and hit testing consistently.
- Row 89: focus mode dims other paths; locked paths/glyphs reject edits with a
  visible reason; lock state persists.

## Owned files

`app/src/editor/**`; `app/src/views/glyph/**` **except** these panel files owned
by other tasks: `panels/{Construction*,constructionCommands.ts,constructionGeometry.ts,ReusableConstructionEditor.tsx,NodeLinksPanel.tsx,PenPointFields.tsx,LayerEffect*,layerEffects*,Smart*,smartComponent*,SourceLayerEditor.tsx,ImagePanel.tsx,ImageTracePreview.tsx,imagePanel.module.css,useImageWorkflow.ts,Cjk*,cjk*,UnihanPanel.tsx,GlyphDatabasePanel*,GlyphTagsField.tsx,glyphTags.ts,LayerColorField.tsx,KerningGroup*,kerningGroup*,useKerningGroupQuery.ts,MetricsPanel.tsx,metricsKeyCommands.ts,useResolvedMetrics.ts,VerticalMetricsPanel.tsx,GlyphVariationPanel.tsx,BracketConditionFields.tsx,bracketConditions.ts}` and `drawVariationPreview.ts`, `SmartCornerField.tsx`;
`app/src/canvas/{glyphPaint.ts,pathCache.ts,Viewport.ts,paintStats.ts}`;
`crates/fc-doc/src/command/{outline/**,outline.rs,layers.rs,editor_metadata.rs,component_locks.rs}`;
`crates/fc-doc/src/problems/**`; `crates/fc-model/src/{contour.rs,guide.rs,anchor.rs,layer.rs}`;
`crates/fc-geom/src/{edit/**,edit.rs,harmonize.rs,convert.rs,corners.rs}`;
`crates/fc-api/src/handlers/{editing.rs,problems.rs}`, `crates/fc-api/src/types/{editing.rs,problems.rs}`;
`app/src/dialogs/{FindProblemsDialog*,SimplifyDialog.tsx}`; `app/src/test/toolContext.ts`;
`app/e2e/draw.spec.ts`, `app/e2e/glyphview.spec.ts`; `docs/p5f-c-editor-acceptance.md`; user docs for the editor.

Shared files (narrow patches only): `crates/fc-model/src/{glyph.rs,component.rs}`,
`crates/fc-doc/src/fcf/schema.md`, `crates/fc-api/src/handlers/glyphs.rs`,
`crates/fc-api/src/types/glyphs.rs`, `app/src/stores/{editorStore,prefsStore}.ts`,
`app/src/shell/{builtinCommands,shortcuts,toolShortcuts}.ts`,
`app/src/dialogs/PreferencesDialog.tsx`, `app/src/i18n/en.json`,
`crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`.

## Gap analysis

Method: read the current code for every row, ran the pending Rust targets the
earlier batch left unexecuted, and ran the full focused frontend suite for the
owned directories as a baseline. Baseline evidence collected before any change
in this task:

```text
cargo test -p fc-geom --test harmonize --test extrema --test direction --test simplify
  harmonize 5 passed · extrema 5 passed · direction 5 passed · simplify 3 passed
cargo test -p fc-doc --test super_smooth --test path_operations --test layer_workflows \
  --test editor_metadata --test component_locks --test problems --test outline_commands \
  --test delete_points --test layers
  10 + 7 + 3 + 16 + 4 + 8 + 4 + 9 + 10 = 71 passed, 0 failed
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 src/editor src/views/glyph
  Test Files 86 passed (86) · Tests 818 passed (818)
```

That closes every "pending central Rust run" note in
`p5-outline-node-semantics-acceptance.md`, `p5-path-operations-acceptance.md`,
`p5-layer-command-workflows-acceptance.md` and
`p5-89-editor-metadata-backend-acceptance.md`: those targets pass on this
checkout.

| Row | Works now                                                                                                                                               | Missing                                                                                                                       | Decision                                                                                                                           |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| 20  | 15 tools registered, 11 with state machines; Text is live (selecting it enters the Edit tab's text mode via `TextView`); Lasso/Eraser/Annotation tested. | No **Pixel** tool at all; the registry doc comment wrongly calls Text inert.                                                   | Implement a Pixel palette entry (`X`) that hands the glyph over to the strike editor; fix the comment; add regression tests.        |
| 21  | Tap/hold borrow with `HOLD_BORROW_MS`, live keymap overrides, palette dotted state.                                                                     | Browser evidence only.                                                                                                        | Verify in the headless browser.                                                                                                    |
| 22  | Double-click/Return smooth toggle, Alt outline-preserving variant, kernel `SetSmooth`.                                                                   | Browser evidence only.                                                                                                        | Verify.                                                                                                                            |
| 23  | `SetSuperSmooth`/`HarmonizeNodes` commands, G2 markers, projection on drag; 17 + 4 tests.                                                                | Browser evidence only.                                                                                                        | Verify.                                                                                                                            |
| 24  | `modifiersOf` separates physical Ctrl from Mod; Ctrl suspends snapping; 1/10/100 nudges.                                                                 | Browser evidence only.                                                                                                        | Verify; record that macOS/Windows native input is untestable here.                                                                  |
| 25  | Proportional handle nudge in `gestures/drag`, `nudgeRun`.                                                                                                | Browser evidence only.                                                                                                        | Verify.                                                                                                                            |
| 26  | Shift-click segment insertion, `AddExtrema`, `ForceExtrema` (`Mod+Alt+Shift+E`).                                                                         | Browser evidence only.                                                                                                        | Verify.                                                                                                                            |
| 27  | Tidy, Correct Direction, both all-masters variants, Reverse, Break, Join, Set Start Point.                                                               | **Open Corner** and **Reconnect Nodes** are absent (the earlier report records them as unimplemented).                         | Implement both in `fc-geom`, `fc-doc` and the Path menu with undo, invalid-input rollback and `.fcf` reopen.                        |
| 28  | Info box X/Y/W/H, nine-point origin, proportional lock, Tab traversal, arrow increments.                                                                 | Browser evidence only.                                                                                                        | Verify.                                                                                                                            |
| 29  | Cursor HUD: delta, angle, length, marquee size, stem width, snap name.                                                                                   | Browser evidence only.                                                                                                        | Verify.                                                                                                                            |
| 30  | Layer and master guides, name, lock, angle field, ruler Alt-drag, measurement toggle, intersection distances.                                            | **LSB/RSB anchoring** is not in the model or the inspector.                                                                    | Add `Guide.anchor` (`Lsb`/`Rsb`) resolved everywhere a guide is drawn, hit, snapped or measured; schema + inspector control.        |
| 31  | Nine independent family toggles, zones, background, oblique guides, ghost component nodes, grid subdivision, one global snap and one hit distance.       | The scope note requires **one distance per snap target**; only two global distances exist.                                     | Add per-target snap distances (0 = inherit the global value) to `SnapPrefs`, the snapping engine and the Preferences dialog.        |
| 32  | Show, `Mod+B` edit, selection→background, copy, swap, per-master, backup.                                                                                | No **Clear** entry point (only the generic layer-row `×`).                                                                     | Add a `layer.clearBackground` command and a panel button.                                                                           |
| 33  | Auto-align toggle, decompose, reorder, lock with kernel enforcement (4 Rust tests).                                                                      | Browser evidence only.                                                                                                        | Verify.                                                                                                                            |
| 34  | Mark-cloud preview with paginated Mark inventory, 11 tests.                                                                                             | Browser evidence only.                                                                                                        | Verify.                                                                                                                            |
| 35  | Always-on active-layer audit, inline markers, hover detail, Fix/Fix All, three checks, session-only toggles.                                             | Only one of the three checks is fixable, and the toggles do not persist.                                                       | Add missing-extremum and zero-length-segment checks, fixes for open contours, coincident points and extrema, and persisted toggles. |
| 36  | Masters/backups/background/custom rows, add, duplicate, rename, remove, colours.                                                                        | Browser evidence only.                                                                                                        | Verify.                                                                                                                            |
| 81  | DPR-aware canvas, CSS-pixel hit testing, `hit_distance`, `node_size`, `handle_size` preferences applied to drawing.                                     | The **handle/node size preference does not reach hit testing**, so a large marker is still grabbed at the small radius.        | Make the hit radius the larger of the hit distance and the drawn marker for nodes and handles.                                      |
| 89  | Path focus, path and glyph locks, lock-aware tools, 10 Rust tests.                                                                                      | Browser evidence only.                                                                                                        | Verify.                                                                                                                            |
| 90  | Measure tool, rulers, intersection spans, italic-aware deltas, `G` drops a guide mid-drag.                                                               | Browser evidence only.                                                                                                        | Verify.                                                                                                                            |
| 91  | Six annotation kinds with per-layer `user_data` storage, inspector editing, 8 tests.                                                                     | Browser evidence only.                                                                                                        | Verify.                                                                                                                            |

Decisions taken without asking, per the protocol:

1. **`crates/fc-api/src/types/session.rs` and `handlers/prefs.rs` belong to task
   `p5f-a`,** but row 31's per-target snap distance cannot exist without a
   `SnapPrefs` field. The change is strictly additive (`#[serde(default)]`, zero
   means "inherit the global distance"), so it is applied as a narrow patch and
   named in the final report for task `a`'s awareness.
2. **`crates/fc-doc/src/command.rs` is unowned** by every `p5f-*` task; the two
   new outline commands are added there as a narrow additive patch.
3. **Open Corner geometry.** The corner node is replaced by two nodes placed
   along the two adjacent tangents *past* the corner, so the contour crosses
   itself in a small triangle. Removing the overlap restores the original sharp
   corner exactly, which is what "opens the corner into an overlapping
   construction" has to mean for it to be reversible.
4. **Reconnect Nodes** covers both readings: two endpoints of open contours are
   joined (delegating to the existing join), and two nodes of one contour are
   merged at the intersection of their outer tangents, which is the exact
   inverse of Open Corner.
5. **Pixel tool.** Pixel editing lives in the strike workspace (task `g`), so
   the palette entry performs the handover and restores the previous tool rather
   than pretending to draw pixels on the outline canvas.

### Corrections made while implementing

6. **Row 33 was mis-assessed above.** Reading the code again showed that
   `Component.auto_align` was honoured as a drag lock and by metrics
   inheritance but that *nothing derived a component's position from an anchor
   pair*: `drawComponents.componentLocked` even documents behaviour
   ("takes its position from the base glyph's anchors") that did not exist.
   `Command::AlignComponents` now performs that placement, the Components panel
   runs it when auto-align is switched on, and the Glyph menu runs it on
   demand. Continuous re-alignment when a *base glyph's* anchors move still
   needs a hook in `crates/fc-doc/src/command/structure.rs` (task `p5f-b`) or a
   post-command pass in `apply.rs` that widens the undo snapshot to the
   dependent glyphs, so row 33 is reported Partial with that exact piece named.
7. **Row 35's "runs live while editing (debounced)" was not met.** The audit
   recomputed only from the fetched outline and hid its markers during a
   gesture. It now reads the canvas's optimistic preview, debounced by 120 ms,
   and keeps the markers up while refusing repairs against geometry the kernel
   has not accepted.
8. **Row 20's "by the palette command" was not met**: no command existed per
   tool. One is registered per tool, deliberately without a keyboard shortcut so
   the tap/hold grammar keeps the key.
9. **Task `p5f-d` asked for three beam pointer hooks** in `GlyphCanvas.tsx`
   during the wave; they are implemented as a small state machine with its own
   tests, and `TextCanvasRun` gained the three optional callbacks.
10. **A real layout defect, first misdiagnosed as a Playwright quirk.** The
    pre-existing `draw.spec` pen test and this task's Open Corner spec both
    failed because pressing `p` did not select the Pen. The cause was not the
    key: the Edit tab's chrome squeezed the canvas — a flex item with
    `min-height: 0` — to about 140 px at 1280x720 and to nothing at 1280x577, so
    the focus click landed in a field and `useToolShortcuts` rightly ignored the
    key. The glyph stage now keeps a 240 px floor, the specs click the canvas
    centre and assert the tool changed, the Open Corner spec draws clear of the
    floating Info box, and both specs pass. The column that has to scroll once
    the chrome no longer fits is `.view` in `app/src/views/text/
    TextView.module.css`, which belongs to task `d`; the needed rule is named in
    the acceptance document.
