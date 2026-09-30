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
