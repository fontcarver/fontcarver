# p5f-j: Construction, images, optical metrics and virtual masters

Area id: `j-construction`. Ports: API 8810, UI 1510. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                            | State at start                                                                                                                                                      |
| --- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 82  | Smart Components / variable components with per-instance axis sliders and smart handles [CJK critical] | `SmartComponentControls`, `SmartAxisSlider`, `SmartComponentHandle`; see `docs/p5-smart-component-controls-acceptance.md`; main-canvas handles to verify/implement. |
| 83  | Corner, cap, segment and brush components (`_corner.*`, `_cap.*`, `_segment.*`, `_brush.*`)            | `fc-geom/construction.rs`, `ConstructionPanel`; see `docs/p5-construction-acceptance.md`; all four types to verify.                                                 |
| 85  | Node-linked guides (Power Guides) with a magnet tool and automatic stem detection                      | `NodeLinksPanel`, `fc-model/node_links.rs`; magnet tool and stem detection to verify/implement.                                                                     |
| 86  | Servant nodes (X/Y interpolated positions)                                                             | `node_links`; see `tasks/p5-85-linked-nodes*.md`; verify dependency updates and cycles.                                                                             |
| 87  | Non-destructive strokes with pen points (width, height, angle, position, progression) and caps/joins   | Partial; see `docs/p5-variable-pen-*-acceptance.md`; independent variable-width export acceptance missing.                                                          |
| 92  | Background images with relative paths, transform, lock, and broken-link indicators                     | `ImagePanel`, `useImageWorkflow`, `fc-doc/image_paths.rs`; see `docs/p5-image-workflows-acceptance.md`.                                                             |
| 93  | Autotrace from raster images                                                                           | Polygonal trace exists (`fc-raster/trace.rs`); curve fitting to implement.                                                                                          |
| 94  | Auto-spacing (optical metrics with a global density factor) and auto-kerning                           | `OpticalSpacingDialog`, `fc-doc/optical`; see `docs/p5-optical-*-acceptance.md`; auto-kerning to verify/implement.                                                  |
| 97  | Virtual masters                                                                                        | `VirtualMastersEditor`; see `docs/p5-virtual-masters-acceptance.md`; verify export and editing.                                                                     |

Existing records: `docs/p5-construction-acceptance.md`, `docs/p5-smart-component-controls-acceptance.md`,
`docs/p5-variable-pen-kernel-acceptance.md`, `docs/p5-variable-pen-ui-acceptance.md`,
`docs/p5-image-workflows-acceptance.md`, `docs/p5-optical-spacing-acceptance.md`,
`docs/p5-optical-ui-acceptance.md`, `docs/p5-virtual-masters-acceptance.md`,
`docs/p5-layer-effects-acceptance.md`, `docs/p5-smart-corners-acceptance.md`,
`docs/superpowers/plans/tasks/p5-85-linked-nodes*.md`, `p5-virtual-masters.md`, `p5-bitmap-trace.md`.

## Scope notes

- Row 82: smart component handles drawn on the main canvas (coordinate the
  draw hook with task c's `GlyphCanvas` through one narrow patch) that drag the
  per-instance axis values; sliders in the inspector; export interpolates
  correctly (FontTools check of a CJK-style fixture).
- Row 83: each of `_corner`, `_cap`, `_segment`, `_brush` attaches to the
  documented geometry (corner nodes, open path ends, segments, path as brush
  spine), updates live when the host or the component changes, rejects cyclic or
  invalid references, bakes and exports; browser evidence for each type.
- Row 85/86: linking a guide to nodes (magnet tool drags a guide onto nodes,
  or select nodes then "link guide"); automatic stem detection proposes stems
  from parallel segments and creates linked guides; servant nodes follow X/Y
  interpolation between their references; cycles rejected; all persist.
- Row 87: independent verification of variable-width strokes: export and
  compare rendered outlines against the source stroke geometry with FreeType,
  covering width/height/angle/progression and each cap/join.
- Row 92: images stored with relative paths, transform (move/scale/rotate/
  skew with handles), lock, broken-link indicator with relink action; per-master.
- Row 93: replace the polygonal trace with curve fitting (corner detection,
  cubic fitting with error tolerance, hole handling, winding); options for
  threshold, smoothing, corner angle, min area; compare against the pixel-edge
  result on fixtures; document limits (photographic input) honestly after
  implementing the vector-quality path on clean scans.
- Row 94: optical spacing with a global density factor and per-glyph
  preview; auto-kerning proposals from optical bounds with a review table
  (accept/reject) and explicit application; Latin and CJK fixtures; master
  scope.
- Row 97: virtual masters as axis-only masters (no outlines) editable in the
  design space; export verified with FontTools (`fvar`/`avar`/`gvar` consistency).

## Owned files

`crates/fc-geom/src/{construction.rs,smart.rs,stroke.rs,variable_pen.rs,effects.rs,style/**,style.rs,filled.rs,boolean.rs}`;
`crates/fc-model/src/{construction.rs,node_links.rs,smart.rs,effects.rs}`;
`crates/fc-doc/src/{command/node_links.rs,image_paths.rs,optical/**}`;
`crates/fc-raster/src/{trace.rs (image autotrace parts),source_image.rs}`;
`crates/fc-api/src/handlers/{constructions.rs,image.rs,optical.rs,effects.rs,style.rs}`;
`crates/fc-api/src/types/{constructions.rs,image.rs,optical.rs,effects.rs,style.rs}`;
`app/src/views/glyph/panels/{Construction*,constructionCommands.ts,constructionGeometry.ts,ReusableConstructionEditor.tsx,NodeLinksPanel.tsx,PenPointFields.tsx,LayerEffect*,layerEffects*,Smart*,smartComponent*,SourceLayerEditor.tsx,ImagePanel.tsx,ImageTracePreview.tsx,imagePanel.module.css,useImageWorkflow.ts}`,
`app/src/views/glyph/SmartCornerField.tsx`; `app/src/canvas/sourceImage.ts`;
`app/src/views/font/{Optical*,opticalSpacing*,opticalMetrics*}`; `app/src/views/fontinfo/VirtualMastersEditor.tsx`;
`app/src/views/style/**`; `app/src/dialogs/ExpandStrokeDialog.tsx`;
`scripts/p5_layer_effects_*.py`, `scripts/p5_style_*.py`, `scripts/p5_smart_corners_*.py`;
`docs/p5f-j-construction-acceptance.md`; user docs for construction, images, optical spacing.

Shared files (narrow patches only): `crates/fc-model/src/{glyph.rs,component.rs,guide.rs}`,
`crates/fc-doc/src/fcf/schema.md`, `crates/fc-compile/src/**` (bake-on-export hooks),
`app/src/views/glyph/{GlyphCanvas.tsx,glyphScene.ts,drawComponents.ts,drawGuides.ts}`,
`app/src/editor/tools/registry.ts` (magnet tool registration), `app/src/shell/builtinCommands.ts`,
`app/src/dialogs/DialogHost.tsx`, `app/src/i18n/en.json`,
`crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`.

## Gap analysis

Written 2026-10-01 after reading the protocol, `docs/STATUS.md`, `docs/TODO.md`,
the nine audit rows, the `p5-resume-audit.md` bounded checklist, every
acceptance document this task file names, `docs/p5f-wave1-checkpoint.md`,
`docs/p5f-c-editor-acceptance.md`, `docs/p5f-f-export-acceptance.md`, and the
code of each area. Baseline: `cargo build -p fc-devserver` succeeds at `5bd842e`
and `cargo test -p fc-api --test optical --test image --test constructions
--test smart_component_editing` passes 6+6+6+3.

### Row 82 — Smart components, per-instance axis sliders, smart handles

Works: `Glyph.smart_axes` + `Component.smart` in the model; interpolation in
`fc_geom::flatten_smart_component` and `fc_model::smart_layer`; per-instance
sliders, numeric fields, a two-axis parameter plane and a keyboard-reachable
handle in `SmartComponentControls`/`SmartAxisSlider`/`SmartComponentHandle`,
reached from a selected row of the Components section; one `SetComponent` per
gesture; `crates/fc-api/tests/smart_component_editing.rs` covers independent
instances, invalid-edit atomicity, undo/redo and native reopen.

Missing: the row asks for _smart handles_, and
`docs/p5-smart-component-controls-acceptance.md` states explicitly that
"direct manipulation handles on the main glyph canvas are not implemented here;
row 82 should not be marked complete on this work alone". No export check of an
interpolated smart component exists.

Will implement: main-canvas handles for the selected smart component — one
draggable handle per local axis, drawn on the component's own bounds, with live
preview and one `SetComponent` commit per drag; keyboard equivalent kept in the
inspector. Plus a FontTools check that a CJK-style smart component exports the
interpolated outline, not the default one.

### Row 83 — `_corner` / `_cap` / `_segment` / `_brush` components

Works: all four `ConstructionKind` variants with their documented attachment
geometry, validation of carrier topology (interior node, open endpoint, two
distinct on-curve nodes on one contour, brush spacing), scale/spacing/length
bounds, arc-length brush stamping, carrier suppression, live re-evaluation
through `fc_model::query::flatten_*_with_sources`, cycle and missing-source
rejection inherited from `flatten_inner`, bake and export through the shared
evaluator, `.fcf` and UFO persistence, and a preview/commit editor
(`ReusableConstructionEditor`).

Missing: (a) the naming convention in the row text (`_corner.*`, `_cap.*`,
`_segment.*`, `_brush.*`) is nowhere in the code — the source picker offers
every glyph with no indication of which ones are meant for which attachment;
(b) no browser evidence for any of the four types;
(c) `docs/p5-construction-acceptance.md` states its Rust suites were never run.

Will implement: a kernel-side convention (`ConstructionKind::glyph_prefix`) and
a source picker that offers the conventionally named glyphs for the chosen mode
first, without forbidding other names (documented); browser evidence for each
of the four types; run the existing Rust suites.

### Row 85 — Node-linked guides with a magnet tool and stem detection

Works: `NodeLinkedGuide` ownership with signed normal offsets, one or two
nonparallel guides per node, guide motion driving its nodes, `LinkToGuide` /
`ProjectToGuide` / `UnlinkGuide` commands, automatic straight stem detection
(`detectStraightStems`) with counter/whitespace rejection and a previewed
count, centre-line stem guides binding both edge node sets, persistence and
validation.

Missing: the magnet _tool_. Linking is an inspector button over the current
selection; `docs/p5-construction-acceptance.md` says "no dedicated drag magnet
tool … is claimed". The tool strip has no magnet entry.

Will implement: a Magnet tool in the tool strip that drags an existing layer
guide and links every node inside a magnet radius when the drag ends — the
guide move and the link land as one undo entry; hovering shows which nodes
would be caught. The inspector route stays.

### Row 86 — Servant nodes (X/Y interpolated)

Works: `NodeLink` with axis, ratio and adjacent-handle ratios; resolution with
cycle and depth rejection; ratios outside 0..1; set/remove per axis from
`NodeLinksPanel`; node identity survives reordering; `.fcf` persistence;
16 document tests in `crates/fc-doc/tests/node_link_commands.rs`.

Missing: only browser evidence that a boundary-node drag moves its servant, and
that a cycle is refused with the document intact.

Will implement: verification, no new model work expected.

### Row 87 — Non-destructive strokes with pen points

Works: everything `docs/p5-variable-pen-kernel-acceptance.md` and
`docs/p5-variable-pen-ui-acceptance.md` record — model, geometry, editor, bake,
native reopen, 2–64 points, width/height/angle/progression, three caps, three
joins.

Missing: exactly what the audit says — "broader independent export acceptance".
No check compares an _exported_ variable-width stroke against the source stroke
geometry outside our own geometry code.

Will implement: an independent check that exports a variable-pen glyph to TTF
and OTF and compares the rendered outline against the analytically expected
stroke envelope with FreeType and FontTools, over width, height, angle,
progression and each cap/join.

### Row 92 — Background images

Works: per-layer (therefore per-master) `Layer.background_image`; embedded bytes
or document-relative links; import from upload or path; six-coefficient affine
placement; lock; undoable remove; relative-path rebasing on Save As; missing or
corrupt source reported without losing the placement; refresh; 50 % canvas
paint; trace with generation + source-hash guards.

Missing: (a) placement is a numeric affine matrix only — the row asks for
move/scale/rotate/skew **with handles**; (b) a broken link shows a raw kernel
message with no dedicated relink action and no indication of the path that was
tried; (c) no browser evidence.

Will implement: on-canvas placement handles (move, corner scale, edge skew,
rotation) that commit one `image.set` per gesture and respect the image and
glyph locks; a broken-link indicator naming the resolved path with a Relink
action; browser evidence.

### Row 93 — Autotrace from raster images

Works: bounded PNG decoding (16 MiB, 4096/side, 1 Mpx, palette/16-bit/alpha,
APNG rejected), luminance-over-white thresholding, exact pixel-edge rings with
holes and correct winding, Douglas–Peucker simplification with a topology
guard, affine placement, preview/apply with generation + SHA-256 guards.

Missing: curve fitting. `trace_image` emits straight lines only; there is no
corner detection, no cubic fitting, no error tolerance, no smoothing control
and no minimum-area filter. `docs/p5-image-workflows-acceptance.md` states
"Bezier curve fitting are not implemented".

Will implement: a real autotrace — resampled ring, corner detection by turn
angle, adaptive least-squares cubic fitting against an error tolerance
(reusing `fc_geom::fit_span`), hole handling and winding preserved, minimum-area
rejection, options for threshold, smoothing, corner angle, error tolerance and
minimum area; node-count and max-error reporting; fixtures comparing the fit
against the analytic shape and against the pixel-edge result. Photographic
input stays a documented limit only after this is verified on clean scans.

### Row 94 — Auto-spacing and auto-kerning

Works: 256-scanline silhouette sampling, UPM-scaled global density 0.25–4,
one/all-master scope, dependency-ordered composites, lock/whitespace/mark/
metrics-key skips, class-rule and explicit-zero protection, one validated table
merge per master, preview invalidation, revision-checked atomic apply/undo, a
numeric and outline review of the first sixteen changes.

Missing: the row's review workflow. Apply is all-or-nothing: the dialog shows
at most sixteen examples and applies **every** proposal. The audit and the
scope note ask for "auto-kerning proposals … with a review table
(accept/reject) and explicit application".

Will implement: the full proposal list in the result (bounded), a review table
with per-row accept/reject and select-all/none, and an apply that sends only the
accepted keys; a key that is no longer in the recomputed proposal is rejected
with the document untouched. Latin and CJK fixtures.

### Row 97 — Virtual masters

Works: named `Font.virtual_masters` positions, staged in the design-space
transaction with axes/masters/instances; validation (absent axis, out-of-range,
duplicate name, duplicate physical/virtual position, 1024 limit); axis-rename
migration; a brace source created at a virtual position; subset trimming and
static-instance cleanup in `fc-compile`; `.fcf` round trip; undo snapshots.

Missing: export verification. `docs/p5-virtual-masters-acceptance.md` says row
97 is "implemented but not accepted as complete" and claims no browser
verification; nothing checks `fvar`/`avar`/`gvar` consistency with FontTools.

Will implement: verification only — browser editing evidence and a FontTools
check that a sparse source at a virtual position contributes a `gvar` delta at
that location while `fvar`/`avar` stay consistent and no phantom master appears.

### Decisions recorded here rather than asked

1. **New files.** Rows 82, 85, 92 and 93 need code that has no owned file yet.
   New files are created under names inside this task's areas
   (`crates/fc-raster/src/autotrace.rs`,
   `app/src/views/glyph/panels/smartComponent*.ts`,
   `app/src/views/glyph/panels/imagePlacement*.ts`,
   `app/src/editor/tools/magnet.ts`,
   `app/src/views/font/opticalProposals*.ts`) and are listed in the report.
2. **Magnet tool wiring.** The task pre-authorises `app/src/editor/tools/registry.ts`
   for "magnet tool registration". A tool cannot reach the canvas without one
   `createTool` arm in `app/src/editor/toolHost.ts`, one guide source line in
   `app/src/editor/editorController.ts` and one optional context field in
   `app/src/editor/tools/toolTypes.ts`. Those three are taken as additive
   one-hunk patches and reported as shared files touched.
3. **`_corner.*` naming is a convention, not a rule.** Enforcing the prefix
   would break every existing document that attaches an ordinary glyph, so the
   prefix orders and labels the source picker and is documented, while any glyph
   remains attachable.
4. **Photographic autotrace** stays out of scope as a documented limit, but only
   after clean-scan curve fitting is implemented and verified, as the task
   requires. Colour segmentation and multi-level posterisation are not attempted.
5. **Task f's follow-up** (`extra`/`onExtra` for `BracketConditionFields` in
   `SourceLayerEditor.tsx`) is implemented here, since that file is owned by
   this task.
