# P5 completion wave 2, task j: construction, images, optical metrics and virtual masters

Date: 2026-10-01. Branch `p5-color-authoring`, from `5bd842e`. Audit rows 82,
83, 85, 86, 87, 92, 93, 94 and 97. Protocol:
[`p5f-protocol.md`](superpowers/plans/tasks/p5f-protocol.md); scope, owned files
and the per-row gap analysis are in
[the task file](superpowers/plans/tasks/p5f-j-construction.md).

Nothing was adapted from FontForge and no dependency was added.

## What this task changed

### Row 93 — autotrace with curve fitting (new kernel work)

`crates/fc-raster/src/autotrace.rs` is new. It turns a pixel-edge ring into
cubic segments and straight lines:

1. optional low-pass filtering of the ring (`smoothing`, 0–8 source pixels);
2. a digital-straightness sweep that tiles the ring with its maximal straight
   runs — those runs, not a Douglas–Peucker polygon, are the structural polygon,
   because a DP polygon of a pixel staircase invents corners out of quantisation
   noise;
3. a corner test on the turn measured over a window proportional to the
   drawing's own size (`corner_angle`, 0–180°), and a straight run long enough
   to be meant as an edge becomes a line after being trimmed until its own chord
   is faithful;
4. least-squares cubic fitting (`fc_geom::fit_span`) of every remaining piece,
   split at its worst sample while splitting still removes at least a fifth of
   the deviation.

`trace_image_with_options` adds the options the row asks for — threshold,
tolerance, curves on/off, corner angle, smoothing and minimum ring area — and
returns the node count, the curve-segment count, the largest deviation, the
rings the area filter dropped and whether any ring kept its exact pixel edges.
Hole handling and winding come from the existing exact pixel tracer and are
re-checked after fitting: a fitted ring that would invert, merge or cross is
refused and its exact edges are kept.

`image.trace` carries the new options and the new report. `image.state` now also
reports the absolute path a link resolved to.

The Source image section gained the options, a quality line, a broken-link panel
that names the path and relinks it, an **Adjust on canvas** mode and
keyboard-reachable move/scale/rotate/skew steps.

### Row 94 — a review table that accepts or rejects single proposals

`fc_doc::optical` now returns every proposed change (`proposals`), not only the
sixteen illustrated examples, and `OpticalOptions.accepted` applies exactly the
reviewed rows. A selection the recomputed proposal no longer contains is an
error, so a review of a stale document can never apply something it did not
show; a rejected spacing row is also not applied to the private snapshot, so the
composites that depend on it keep measuring the unchanged glyph.
`OpticalSpacingDialog` shows the table with per-row accept boxes, accept/reject
all, a windowed view for large proposals, and an apply that sends the accepted
keys.

### Rows 82, 85, 92 — three new direct-manipulation surfaces

- **Smart-component handles on the main canvas** (`smartComponentCanvas.ts`,
  `smartComponentPointer.ts`): a selected component whose base glyph has local
  axes gets one track per axis on its own bounds; dragging a handle sets that
  instance's coordinate with the value shown beside it and writes one
  `SetComponent` per drag. The first four axes get a track; a locked component
  gets none.
- **The Magnet tool** (`app/src/editor/tools/magnet.ts`, key `M`): pressing a
  layer guide picks it up, the on-curve nodes within the magnet radius of where
  it now lies are marked, and releasing writes the guide move and the links as
  one undo entry. Alt projects the caught nodes onto the guide first. A master
  guide, a locked guide and empty canvas each say why they did nothing.
- **Source-image placement handles** (`canvas/sourceImagePlacement.ts`,
  `imagePlacementPointer.ts`, `imagePlacementSession.ts`): move, four corner
  scales, four edge skews and a rotation knob, with Shift for a uniform scale
  and a 15° rotation snap, each gesture one `image.set`.

### Row 83 — the naming convention is real

`fc_model::CONSTRUCTION_PREFIXES` and `ConstructionKind::glyph_prefix` name the
`_corner.`, `_cap.`, `_segment.` and `_brush.` convention, the construction
editor offers the glyphs with the right prefix for the chosen kind, and a source
named otherwise is pointed out rather than refused. Documented in
`fcf/schema.md`; no stored model field changed.

### Task f's follow-up

`SourceLayerEditor.tsx` now passes `extra`/`onExtra` to
`BracketConditionFields`, so a compound alternate-layer condition can be edited
from the source dialog and not only from the variation panel.

## Commands and their output

Rust, each under `flock .scratch/p5f/cargo.lock`, `TMPDIR` inside the task
scratch:

```text
cargo test -p fc-raster                     28 + 10 + 8 + 2 + 3 + 7 passed, 0 failed
cargo test -p fc-model  --test construction_references --test smart --test smart_corners
                                            3 + 4 + 2 passed, 0 failed
cargo test -p fc-geom   --test constructions --test effects --test variable_pen --test smart_corners
                                            3 + 6 + 9 + 8 passed, 0 failed
cargo test -p fc-doc    --test constructions --test construction_persistence
                        --test node_link_commands --test virtual_masters
                        --test variable_pen --test image_paths
                                            3 + 2 + 8 + 3 + 4 + 4 passed, 0 failed
cargo test -p fc-doc --lib optical           3 passed, 0 failed
cargo test -p fc-api    --test constructions --test image --test optical
                        --test smart_component_editing
                                            4 + 9 + 9 + 3 passed, 0 failed
cargo clippy -p fc-raster -p fc-model -p fc-doc -p fc-api --all-targets -- -D warnings   clean
rustfmt --edition 2021 --check <owned Rust files>                                        clean
```

New Rust tests: 10 in `fc-raster/src/autotrace.rs` (square, circle,
high-resolution circle, rounded rectangle, stem, corner angle, tolerance
monotonicity, smoothing floor, degenerate input), 6 in
`fc-raster/src/source_image.rs` (curve fit against the pixel-edge result, holes
and winding, minimum area, invalid options, placement, diagonal contacts), 3 in
`fc-api/tests/image.rs` (curve report and write, minimum area and invalid
options, resolved path and relink) 3 in `fc-api/tests/optical.rs` (accepted subset,
empty and stale acceptance, accepted kerning subset), 1 in
`fc-model/tests/construction_references.rs` (the naming convention) and, in the
review round, 2 in `fc-doc/tests/node_link_commands.rs` (the guide lock).

Frontend:

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/glyph/panels src/canvas src/editor/tools \
  src/views/font/OpticalSpacingDialog.test.tsx src/shell/commandHelp.test.ts \
  src/shell/toolShortcuts.test.ts src/shell/keymapPresets.test.ts \
  src/shell/shellPlatformParity.test.ts
Test Files  59 passed (59)
Tests       466 passed (466)

pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 src/views/glyph
Test Files  68 passed (68)
Tests       605 passed (605)

pnpm --dir app check (tsc -b)   clean for every file this task owns or touched
pnpm --dir app exec eslint <owned dirs and shared files>   exit 0
pnpm --dir app exec prettier --check <owned files, en.json, docs/user>   clean
```

New frontend tests: 9 in `canvas/sourceImagePlacement.test.ts`, 9 in
`editor/tools/magnet.test.ts`, 8 in `panels/smartComponentCanvas.test.ts`, 7 in
`panels/ImagePanel.test.tsx`, 4 in `views/font/OpticalSpacingDialog.test.tsx`,
2 in `panels/SourceLayerEditor.test.tsx` and, in the review round, 2 in
`panels/canvasHandleGuard.test.ts` and 2 in `panels/ConstructionSources.test.ts`.

`src/i18n/i18n.test.ts` is expected to fail until the translation pass: the new
English keys have no zh-CN entries yet.

## Independent verification against the exported fonts

`scripts/p5f_j_construction_acceptance.py` drives a real `fc-devserver` on port
8810 and compares what the kernel produces against a reference this script
computes for itself. Evidence:
[`acceptance/p5f-j-construction.json`](acceptance/p5f-j-construction.json).

```text
python3 scripts/p5f_j_construction_acceptance.py --api http://127.0.0.1:8810 \
  --output .scratch/p5f-j-construction/acceptance
row 87 taper-ellipse:      1400 sampled points agree in TTF and OTF with FontTools and FreeType
row 87 height-only:        1400 …
row 87 angle-rectangle:    1400 …
row 87 smooth-progression: 1400 …
row 87 cap-round:          1400 …
row 87 cap-square:         1400 …
row 87 join-round:         1400 …
row 87 join-miter:         1400 …
row 87 join-bevel:         1400 …
row 93: 36 curve-fitted nodes against 315 pixel-edge nodes, deviation 1.50 px,
        4000 sampled points agree
row 82: exported instance bounds {'A': (0,0,100,100), 'B': (0,0,300,100), 'C': (0,0,100,700)}
row 97: gvar peaks [0.5, 1.0], avar {}, instance widths {100.0: 100, 500.0: 400, 900.0: 400}
```

- **Row 87.** An independent Python stroker sweeps the nib along the source
  centreline — quadrilaterals between the support offsets, join patches at every
  corner, cap patches at the ends — and answers inside/outside by half-plane
  tests, with no polygon library and no curve fitting. Nine cases cover a width
  taper, a height-only taper, a rotated rectangular nib, the smooth progression,
  all three caps and all three joins. Each case exports TTF and OTF through
  `session.export`, samples 1400 positions that the reference places more than
  **one font unit** clear of its boundary, and requires FontTools point-in-path
  **and** FreeType's own rasteriser to agree with the reference at every one of
  them. Zero mismatches in 9 cases × 2 containers × 2 readers × 1400 points. The
  band exists only because the two descriptions quantise differently; at one unit
  on a 1000-unit em it is a tenth of a percent, not a tolerance that could hide a
  wrong cap, join or taper.
- **Row 93.** An analytic shape — an elliptical annulus whose left side is cut
  square, so the trace has to find two corners and two straight runs as well as
  four arcs — is rasterised into a 1000 × 1000 PNG here, traced by the kernel with
  curve fitting, applied, and exported. The fit uses 36 nodes where the
  pixel-edge trace uses 315, of which 12 are straight segments on the cut side,
  its reported deviation is 1.50 source pixels, and 4000 sampled positions more
  than three pixels clear of the analytic shape agree with both FontTools and
  FreeType.
- **Row 82.** A two-axis radical with a source at every corner of its axis grid
  is used by two component instances. The exported outlines are the
  interpolations (300 × 100 and 100 × 700), not the default 100 × 100 source, and
  FreeType confirms the ink boundary of the interpolated instance.
- **Row 97.** A sparse source at the named virtual position produces a `gvar`
  region peaked at that location beside the full-weight region, `fvar` keeps the
  single axis and gains no named instance for the virtual position, and
  instantiating the variable font with FontTools' own instancer at wght 100, 500
  and 900 gives 100, 400 and 400 units of width — the non-linear shape the
  intermediate source defines.

## Browser verification

An isolated `agent-browser` session (`p5f-j`) drove `fc-devserver` on 8810
through Vite on 1510 at 1440 × 1000, on a synthetic document built for this task.
Headless Chrome in this checkout needs the shell sandbox disabled.

| Row | What was done in the browser                                                                                                                                                                                                                                                                                 | Evidence                                                                                    |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------- |
| 82  | A selected smart component drew one track per local axis; dragging the `wide` handle to the end of its track changed the inspector slider from 320 to 520, wrote one `Set smart parameter` entry, and one undo restored `{wide: 320, tall: 300}`.                                                            | [smart-component-handles.png](screenshots/p5f-j-construction/smart-component-handles.png)   |
| 83  | All four attachment kinds were created on one carrier through the editor, each from the conventionally named source the editor offered for that kind (`_corner.serif`, `_cap.round`, `_segment.bar`, `_brush.dot`), each previewed before being kept live.                                                   | [construction-attachments.png](screenshots/p5f-j-construction/construction-attachments.png) |
| 85  | The Magnet tool showed a grab cursor over a layer guide, marked the four stem-foot nodes while the guide was dragged, and committed `Link nodes to guide` as one entry; the guide then sat at y 109.4 with four nodes bound at −9.45. Moving that guide to y 209.4 moved all four nodes from y 100 to y 200. | [magnet-tool.png](screenshots/p5f-j-construction/magnet-tool.png)                           |
| 86  | **X servant** on a selected interior node stored a ratio-1.0 X relationship; dragging the boundary node to x 421 moved the servant to x 421 with it.                                                                                                                                                         | —                                                                                           |
| 92  | The source image drew with its placement frame and nine handles; a corner drag scaled it about the opposite corner (`[1.1,0,0,1.1,20,−20]` → `[1.344,0,0,1.255,20,−20]`, translation unchanged) and the rotation knob turned it to −53.8° keeping the 1.1 scale, each as one `Edit source image` entry.      | [image-placement-handles.png](screenshots/p5f-j-construction/image-placement-handles.png)   |
| 93  | The same image traced to 18 nodes and 16 curve segments with a 1.49-pixel deviation, against 230 nodes and 0 curve segments with **Fit curves** off; applying appended two contours with off-curve points and kept the source image.                                                                         | [autotrace-preview.png](screenshots/p5f-j-construction/autotrace-preview.png)               |
| 94  | Seven glyphs previewed seven spacing proposals; rejecting five left `2 of 7 accepted`, and applying changed only H (700 → 600) and O (700 → 546) in one `Optical spacing and kerning` entry. Every rejected glyph kept its advance.                                                                          | [optical-review-table.png](screenshots/p5f-j-construction/optical-review-table.png)         |
| 97  | The design-space editor listed the virtual position `Middle` at wght 50, and editing it to 70 and applying stored `{"name": "Middle", "location": {"wght": 70}}` in one `Edit design space…` entry.                                                                                                          | [virtual-masters.png](screenshots/p5f-j-construction/virtual-masters.png)                   |

## Documentation

- [`docs/user/construction.md`](user/construction.md) is new: attachments and
  their naming convention, node-linked guides and the Magnet tool, automatic
  stem detection, servant nodes, smart components on the canvas.
- [`docs/user/source-images.md`](user/source-images.md) is new: import, broken
  links and relinking, placement on the canvas and by keyboard, every autotrace
  option, what the reported numbers mean, and the photographic limit.
- [`docs/user/optical-spacing.md`](user/optical-spacing.md) gained the review
  table and the stale-row refusal.
- `crates/fc-doc/src/fcf/schema.md` records the construction naming convention.
  No stored model field changed in this task, so nothing else in the schema moved.

## Limits, stated honestly

1. **Autotrace reads one brightness threshold.** A photograph, a coloured
   drawing or a shaded scan has no single threshold that separates ink from
   paper, so tracing one returns the outline of whatever falls below the
   threshold. Colour segmentation, posterisation and centre-line tracing are not
   implemented. This limit is recorded only now that clean-scan curve fitting is
   implemented and verified above: on a clean raster the fit uses an order of
   magnitude fewer nodes than the pixel edges and agrees with the analytic shape.
2. **The tolerance is a target, not a guarantee.** A one-bit raster of a curve
   keeps a flat run of several pixels at every extreme, so no smooth curve can
   follow such a ring closer than about one pixel however many nodes it is
   given. The trace stops splitting once splitting no longer helps and reports
   the deviation that remains; `smoothing` lowers the floor at the cost of
   detail. Measured: 1.49 px in the browser case, 1.50 px in the analytic case.
3. **Smart-component handles commit on release.** The value and the handle
   follow the pointer live; the component's outline is re-drawn when the gesture
   commits, exactly as the inspector's sliders behave, because resolving a smart
   interpolation is a kernel call. Only the first four local axes get a canvas
   track; the rest stay in the inspector.
4. **The magnet binds on-curve nodes.** The kernel adds each caught node's
   neighbouring handles itself. A node already owned by two non-parallel guides,
   or one that is a servant, is refused by the kernel with the document intact.
5. **Attachment placement is affine.** A corner or cap rotates to the carrier
   tangent, a segment scales to the chord, a brush follows arc length; none of
   them bends the source to the carrier's curvature and none trims the carrier.
   The convention prefix is advisory, never enforced.
6. **Optical spacing stays a sampled approximation.** 256 scanlines, a
   density-scaled clearance target and a sampled clearance floor; it is a
   starting point for proofing, not continuous collision detection or a
   script-aware optimiser. The review table does not change that — it only makes
   the proposal selective.
7. **Virtual masters are axis-only positions.** A position carries no layers,
   metrics or kerning of its own; glyph geometry at that position comes from the
   sparse brace sources created there, and the source-format writers still do
   not preserve the named preset list (already recorded in the schema).
8. **No native desktop run.** Everything above is Linux, in a headless browser
   and through the HTTP transport.

## Recommended audit status

| Row | Recommendation                 | Why                                                                                                                                                                                                                                                                                                     |
| --- | ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 82  | **Complete**                   | Per-instance sliders, a two-axis plane, and main-canvas axis handles that drag the per-instance values with one undo entry each; export verified to carry the interpolated outlines, not the default source. Limit 3 is a bounded behaviour, not a missing feature.                                     |
| 83  | **Complete**                   | All four attachment kinds created in the browser from conventionally named sources, previewed, kept live, re-evaluated, validated against cycles and invalid carriers, persisted and baked on export. Limit 5 is the documented placement model.                                                        |
| 85  | **Complete**                   | A Magnet tool that drags a guide onto nodes and binds them in one undo entry, the inspector route beside it, automatic straight-stem detection with centre-line guides, and a live guide that moves its nodes.                                                                                          |
| 86  | **Complete**                   | X/Y servant interpolation created from the inspector, following its boundaries live, with cycle and ownership rejection and `.fcf` persistence.                                                                                                                                                         |
| 87  | **Complete**                   | The remaining gap was independent export acceptance; width, height, angle, both progressions, all three caps and all three joins now agree with an independent stroker in TTF and OTF under both FontTools and FreeType.                                                                                |
| 92  | **Complete**                   | Relative links, embedded sources, per-layer (so per-master) placement, lock, broken-link indication with the resolved path and relink, and move/scale/rotate/skew with handles plus keyboard equivalents.                                                                                               |
| 93  | **Complete within limits 1–2** | Curve fitting with corner detection, an error tolerance, smoothing, a minimum-area filter, holes and winding, verified against the analytic shape and against the pixel-edge result. The photographic limit is documented after the clean-scan path was implemented and verified, as the task required. |
| 94  | **Complete within limit 6**    | Optical spacing with a global density factor, per-glyph preview, automatic kerning proposals, a review table that accepts or rejects single rows, explicit application, and a stale-row refusal that applies nothing.                                                                                   |
| 97  | **Complete**                   | Virtual masters edited in the design space in one transaction, a sparse source created at a position, and `fvar`/`avar`/`gvar` verified with FontTools including a FontTools-side instantiation at three locations.                                                                                     |

## Shared files touched, all additive

`crates/fc-model/src/{construction.rs,lib.rs}`,
`crates/fc-doc/src/fcf/schema.md`,
`crates/fc-api/src/types/{image.rs,optical.rs}`,
`crates/fc-api/src/handlers/{image.rs,optical.rs}`,
`app/src/views/glyph/GlyphCanvas.tsx`, `app/src/editor/tools/registry.ts`,
`app/src/editor/toolHost.ts`, `app/src/editor/tools/toolTypes.ts`,
`app/src/editor/editorController.ts`, `app/src/editor/tools/registry.test.ts`
(the strip-order list gained `magnet`), `app/src/i18n/en.json`.

The three editor files beyond the pre-authorised `registry.ts` are the minimum a
tool needs to reach the canvas: one `createTool` arm, one optional `guides` field
on `ToolContext`, and one line that supplies it.

## New i18n key namespaces

All under existing namespaces, English only: `imageWorkflow` (30 keys),
`opticalSpacing` (11), `construction` (9 plus two `construction.undo` keys),
`panel.smartComponent` (2), `tool.magnet` and `tool.magnetHint`. The review round
replaced `construction.magnetHud` and `construction.magnetSnapHud` with
`_one`/`_other` pairs, so the delta pass sees four keys there instead of two.

## Review round

The review returned **accept with fixes**. Every item below is fixed, with the
test that proves it; the focused Rust and Vitest suites, Clippy, ESLint, tsc and
Prettier were rerun afterwards and the evidence JSON regenerated.

| #   | Finding                                                                                                                                                                                                                                                                              | What changed                                                                                                                                                                                                                                                                                                                                                                                                 | What proves it                                                                                                                                                                                                                                                                                                                                                                                                 |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Medium (row 85).** `Guide.locked` was read nowhere in `fc-doc`, so `LinkToGuide`, `ProjectToGuide`, `UnlinkGuide`, `SetGuide` and `RemoveGuide` all accepted a locked guide; only the Magnet tool refused, in the UI, so the inspector route and any API client bypassed the lock. | The kernel now refuses all five on a locked guide, with the explicit-unlock escape a locked image has: `SetGuide` takes only the stored value back, or that value unlocked, and the stored identity is compared as it would be written. `crates/fc-doc/src/command/structure.rs` (`locked_guide`, `SetGuide`, `RemoveGuide`) and `crates/fc-doc/src/command/node_links.rs` (`NodeLinkCommand::guide_index`). | `fc-doc/tests/node_link_commands.rs` › `a_locked_guide_refuses_every_change_until_it_is_explicitly_unlocked` (five refusals, document unchanged after each, then unlock and link) and `a_locked_guide_still_moves_the_nodes_it_already_owns`. Re-confirmed live against `fc-devserver`: all five refused with the document byte-identical, the explicit unlock accepted, and the link then stored one binding. |
| 2   | **Low-medium.** `GlyphCanvas` offered the smart-component and image-placement handles for every tool, so a Hand pan, a Zoom click or a space-held pan starting on a handle was captured.                                                                                             | `canvasHandlesActive(tool, spaceHeld)` in `app/src/views/glyph/panels/canvasHandleGuard.ts` now gates both `pointerDown` and `pointerMove`, and the same answer (on the tool alone) decides whether the handles are drawn. The space-held part is deliberately not read inside `draw`: a repaint on every key press is not worth a transient affordance, and the press is routed to the view regardless.     | `panels/canvasHandleGuard.test.ts` — Hand, Zoom and space-held are refused, and `TOOLS` is swept so the refused set is exactly `['zoom', 'hand']`.                                                                                                                                                                                                                                                             |
| 3   | **Low.** `STROKE_BAND = 6.0` let "zero mismatches" tolerate ±6 units.                                                                                                                                                                                                                | The band is now `1.0` font unit (and `TRACE_BAND` 12 → 3 pixels).                                                                                                                                                                                                                                                                                                                                            | The rerun above: nine cases × two containers × two readers × 1400 points, still zero mismatches, at a tenth of a percent of the em.                                                                                                                                                                                                                                                                            |
| 4   | **Low.** `ring_ink` claimed a straight-sided stem but built a plain elliptical annulus, so no corner or straight-run path of `autotrace.rs` was exercised.                                                                                                                           | The fixture is now the annulus intersected with a half plane, so the trace must find two corners and two straight runs beside the four arcs; `trace_reference` is the three-constraint signed distance and `ring_ink` derives from it, so the mask and the reference cannot disagree.                                                                                                                        | The rerun: 36 nodes against 315 pixel-edge nodes, **12 straight segments**, 1.50 px deviation, 4000 sampled points agreeing; the script asserts at least two straight segments.                                                                                                                                                                                                                                |
| 5   | **Low.** The prefix table was duplicated in `fc-model` and in `ConstructionSources.tsx`.                                                                                                                                                                                             | The kernel stays the single source of truth and the editor's copy is pinned to it.                                                                                                                                                                                                                                                                                                                           | `panels/ConstructionSources.test.ts` reads `CONSTRUCTION_PREFIXES` out of `crates/fc-model/src/construction.rs` and compares values and kind order, with a second case showing a drifted table fails.                                                                                                                                                                                                          |
| 6   | **Low.** `DEFAULT_TRACE_SETTINGS.min_area` was 0, so a speckled raster previewed thousands of rings.                                                                                                                                                                                 | The default is 4 square source pixels; zero stays reachable and keeps every ring.                                                                                                                                                                                                                                                                                                                            | `panels/ImagePanel.test.tsx` › "sends every curve option…" asserts the field starts at 4 and still accepts 0. Documented in `docs/user/source-images.md`.                                                                                                                                                                                                                                                      |
| 7   | **i18n.** `construction.magnetHud` and `construction.magnetSnapHud` had no plural form.                                                                                                                                                                                              | Both are now `_one`/`_other` pairs.                                                                                                                                                                                                                                                                                                                                                                          | `editor/tools/magnet.test.ts` › "counts the caught nodes in the singular and the plural" asserts both exact strings.                                                                                                                                                                                                                                                                                           |

Keys added or changed in `app/src/i18n/en.json` for the translation delta pass:
`construction.magnetHud` and `construction.magnetSnapHud` were **removed** and
replaced by `construction.magnetHud_one`, `construction.magnetHud_other`,
`construction.magnetSnapHud_one` and `construction.magnetSnapHud_other`. A
translation pass had already reached the two superseded keys, so `zh-CN.json`
currently holds them and misses the four new ones; `zh-CN.json` was not touched
here. No other key changed in this round.

Commands rerun after the fixes:

```text
cargo test -p fc-doc                     49 test binaries, all ok, 0 failed
cargo test -p fc-raster                   7 test binaries, all ok, 0 failed
cargo test -p fc-api --test constructions --test image --test optical
                     --test smart_component_editing      4 + 9 + 9 + 3 passed
cargo test -p fc-model --test construction_references --test smart --test smart_corners
                                                        3 + 4 + 2 passed
cargo test -p fc-geom --test constructions --test effects --test variable_pen
                      --test smart_corners               3 + 6 + 8 + 9 passed
cargo clippy -p fc-raster -p fc-model -p fc-doc -p fc-api --all-targets -- -D warnings  clean
rustfmt --edition 2021 --check <changed Rust files>                                     clean

pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/glyph/panels src/canvas src/editor/tools \
  src/views/font/OpticalSpacingDialog.test.tsx src/shell/commandHelp.test.ts \
  src/shell/toolShortcuts.test.ts src/shell/keymapPresets.test.ts \
  src/shell/shellPlatformParity.test.ts
Test Files  61 passed (61)
Tests       471 passed (471)

pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 src/views/glyph
Test Files  70 passed (70)
Tests       609 passed (609)

pnpm --dir app check (tsc -b), eslint and prettier over the changed files: clean
python3 scripts/p5f_j_construction_acceptance.py …    complete, 0 mismatches
```

The recommended statuses above are unchanged: every fix tightens an existing
claim or closes a bypass rather than opening a new gap. Row 85's recommendation
now rests on a kernel-enforced lock rather than a UI refusal.
