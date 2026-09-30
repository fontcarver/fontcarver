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
