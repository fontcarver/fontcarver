# p5f-g: Bitmap fonts

Area id: `g-bitmap`. Ports: API 8807, UI 1507. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                                                                 | State at start                                                                                                     |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| 64  | Strikes as first-class layers keyed by (ppem, depth), with per-strike metrics                                                               | Reusable evidence (P3); re-verify.                                                                                 |
| 65  | Strike editor: pencil (toggle-then-match), line, rect, ellipse, fill, eraser, marquee select/move/copy, move bitmap, eyedropper, zoom, hand | Most tools exist; exact toggle-then-match pencil, eyedropper and hand to verify.                                   |
| 66  | Outline overlay, pixel grid, advance column, pointer/delta HUD, units-per-pixel readout                                                     | Grid/baseline exist; overlay, advance column, HUD and readout to verify/implement.                                 |
| 67  | Rasterise from outline (per glyph and batch) with threshold control                                                                         | Reusable evidence; re-verify.                                                                                      |
| 68  | Pixel-to-outline: rectangle merging, collinear removal, correct winding, corner mode (sharp / chamfer / round), diagonal join mode          | Exact boundaries exist; corner and diagonal-join modes to verify/implement (`docs/acceptance/p5-bitmap-trace.md`). |
| 69  | Pixel-as-component mode driven by an editable `pixel` glyph                                                                                 | `BitmapOutlineTools`, `BitmapReferences`; see `docs/acceptance/p5-bitmap-parity.md`.                               |
| 70  | Grid presets guaranteeing integer units per pixel (1000/1024/2048 UPM families)                                                             | `BitmapGridPreset`; verify all families and the guarantee.                                                         |
| 71  | BDF import/export with a full properties editor; PNG sheet import/export                                                                    | `StrikeProperties`, `BitmapSheet*`; see `docs/acceptance/p5-bitmap-sheet.md`.                                      |
| 101 | Multi-strike waterfall and cross-strike copy/scale                                                                                          | `BitmapWaterfall`, `BitmapScale`, `BitmapTransfer`; combined acceptance missing.                                   |
| 102 | EBDT/EBLC strike embedding in TTF; sbix and CBDT/CBLC colour bitmaps                                                                        | Reusable evidence (P3, FontTools-decoded); re-verify with the current writer.                                      |
| 103 | TTF/OTF import with pixel-font auto-detection and strike reconstruction                                                                     | `BitmapReconstruction`; detection heuristics and reconstruction to verify/implement.                               |

Existing records: `docs/p3-acceptance.md`, `docs/acceptance/p5-bitmap-parity.md`,
`docs/acceptance/p5-bitmap-sheet.md`, `docs/acceptance/p5-bitmap-trace.md`,
`docs/superpowers/plans/tasks/p5-bitmap-*.md`.

## Scope notes

- Row 65: pencil toggles the first pixel and then paints every following pixel
  to match that state; eyedropper picks the value under the pointer for
  depth > 1; hand pans; marquee move/copy with modifier; every tool reachable by
  shortcut and palette.
- Row 66: outline overlay of the master at the strike scale, advance column,
  pointer position and drag delta in pixels and font units, units-per-pixel
  readout; toggles persist.
- Row 68: expose corner mode and diagonal join mode as options; test each on
  fixtures (staircase, diagonal, hole); outputs pass the outline audit
  (direction, no self-overlap where the mode promises none).
- Row 69: an editable `pixel` glyph drives the component-based output; editing
  the `pixel` glyph updates all pixel components; export decomposes or keeps
  components per option; FontTools verification.
- Row 70: presets for 1000, 1024 and 2048 UPM families with the ppem values
  that give integer units per pixel; applying a preset sets strike metrics and
  grid; a non-integer choice is rejected with the nearest valid suggestion.
- Row 71: full BDF property editor (typed properties, add/remove/rename
  custom ones, validation) round-tripping through `fc-formats`; PNG sheet export
  and import with a layout description, verified with a pixel-exact test.
- Row 101: waterfall across strikes for text and single glyph; copy/scale a
  glyph between strikes (nearest and area-average) with pixel-exact tests.
- Row 103: on TTF/OTF import, detect embedded bitmaps or pixel-font geometry
  (integer-grid outlines) and offer strike reconstruction; verify on a real
  pixel-font fixture and on a normal font (no false positive).

## Owned files

`app/src/views/bitmap/**`; `app/src/canvas/{bitmapGeometry.ts,bitmapRenderer.ts,bitmapSheetImage.ts}` and tests;
`app/src/editor/tools/pixel.ts` if you introduce it (coordinate the palette registration through `registry.ts` with a narrow patch);
`crates/fc-raster/src/pixel.rs`; the pixel-to-outline parts of `crates/fc-raster/src/trace.rs`
(task j owns image autotrace in the same file; keep patches narrow); `crates/fc-formats/src/bitmap/**`;
`crates/fc-sfnt/src/bitmap/**`; `crates/fc-model/src/{bitmap/**,bitmap.rs}`;
`crates/fc-doc/src/command/bitmap.rs`; `crates/fc-api/src/handlers/{bitmap.rs,bitmap_parity.rs}`;
`crates/fc-api/src/types/{bitmap.rs,bitmap_config.rs,bitmap_parity.rs}`;
`app/e2e/bitmap.spec.ts`; `docs/p5f-g-bitmap-acceptance.md`; user docs for bitmap fonts.

Shared files (narrow patches only): `crates/fc-sfnt/src/read/**` and `write/mod.rs`
(embedding hooks), `crates/fc-doc/src/fcf/schema.md`, `app/src/editor/tools/registry.ts`,
`app/src/shell/{builtinCommands,shortcuts}.ts`, `app/src/i18n/en.json`,
`crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`.
