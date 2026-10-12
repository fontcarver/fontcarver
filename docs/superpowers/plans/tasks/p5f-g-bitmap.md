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

## Gap analysis

Written 2026-10-01 after reading the code for every row (model, commands, API,
`fc-raster`, `fc-formats`, `fc-sfnt`, the whole of `app/src/views/bitmap/`) and
running the baseline suites (`cargo test -p fc-raster` 8+2+7 pass,
`cargo test -p fc-api --test bitmap --test bitmap_parity` 4+4 pass).

### Row 64 — strikes as first-class layers keyed by (ppem, depth)

Works: `BitmapStrike { ppem, depth, ascent, descent, properties, glyphs }` is a
font-level list sorted and keyed by `(ppem, depth)`; `Font::validate_bitmaps`
rejects duplicate keys and enforces the 256-strike and 256 MiB budgets;
`BitmapCommand::{SetStrike, RemoveStrike, SetGlyph, SetGlyphs}` are undoable,
`.fcf` carries them, and `crates/fc-doc/tests/bitmap_commands.rs` covers
undo/redo, rollback, rename/duplicate/delete and native round trip.
Missing: nothing structural. To add: an explicit kernel test that per-strike
metrics and properties survive `.fcf` reopen and that a duplicate `(ppem, depth)`
insert and an out-of-range depth are rejected without touching the document.

### Row 65 — strike editor tools

Works: pencil with toggle-then-match (`pencilInk` picks the ink from the first
pixel and the gesture keeps it), eraser, line, rectangle, ellipse, flood fill,
marquee select and marquee move, whole-bitmap move (arrow keys and the shift
buttons), eyedropper (passive, no command), hand (passive pan, arrow-key pan),
zoom select. Each has a toolbar button.
Missing: (a) **marquee copy** — dragging a selection always moves it, there is no
modifier that stamps a copy; (b) **shortcuts** — no pixel tool has a key;
(c) **palette/menu** — no pixel tool is a registered command, so the Bitmap menu
and `Mod+K` cannot reach them. Implement all three: an owned Zustand store for
the active pixel tool, `bitmap.tool.*` commands in the Bitmap menu with plain-key
shortcuts gated on the bitmap editor being mounted, and Alt-drag copy.

### Row 66 — overlay, grid, advance column, HUD, units-per-pixel

Works: outline overlay of the master at strike scale (`outlinePath` +
`unitsPerPixel`), pixel grid, baseline/origin guides, a dashed advance line, a
pointer/delta readout in pixels, a units-per-pixel readout.
Missing: (a) the HUD gives **pixels only**, the row asks for pixels *and* font
units; (b) the grid and overlay toggles are component `useState`, so they do not
**persist**; (c) the advance is a hairline, not an **advance column**. Implement
all three (persisted toggles in the owned store, a shaded advance column, font
units beside every pixel readout).

### Row 67 — rasterise from outline, per glyph and batch, with threshold

Works: `bitmap.rasterize` takes `glyphs` (empty = every glyph), `threshold`,
`depth`, `ppem` and `hinting`, merges into an existing strike, is one undo step,
and `fc_raster::rasterize_options` rejects threshold 0, ppem 0/>4096 and depth 32.
The UI has the threshold field and an "All glyphs" switch.
Missing: the threshold field can be left empty, which sends `NaN` (JSON `null`)
and silently rasterises at 128. To add: guard the control and add kernel tests
for the rejected thresholds and sizes.

### Row 68 — pixel-to-outline

Works: `fc_raster::trace_bitmap_with_options` with `BitmapTraceMode::{PixelEdges,
ChamferedCorners, RoundedCorners, DiagonalSmoothing}` and `join_diagonals`; the
exact mode merges rectangles and removes collinear points; outside contours are
clockwise and holes counterclockwise; 7 focused tests pass, one exhausting 512
3x3 masks. Both options are in the UI.
Missing: (a) no check that the emitted contours **pass the outline audit** the
rest of the app uses (direction, self-intersection); (b) tracing is one glyph at
a time, while rasterising has a batch, so round-tripping a strike means one
click per glyph. Implement: batch trace in one undo step and an audit-based test
per mode.

### Row 69 — pixel-as-component mode

Works: `bitmap.pixel_components` creates or reuses a `pixel` glyph (a UPM square
in every master), replaces the target layer with one component per inked pixel
scaled by `1/ppem`, sets the advance, and is one undo step; `decompose_components`
already exists as an export option.
Missing: (a) the `pixel_glyph` name is only length-checked, so an invalid name
can create an unusable glyph; (b) no entry point to **edit** the pixel glyph from
the bitmap editor; (c) no test that editing the `pixel` glyph changes every
composite, and no independent (FontTools) check of the two export modes.

### Row 70 — grid presets guaranteeing integer units per pixel

Works: `integerGridPresets` filters a hard-coded ppem list by `upm % ppem === 0`.
Missing: (a) the hard-coded list **misses valid sizes** (1024 and 2048 for UPM
2048, 500 for UPM 1000), so the preset list is not the set the row describes;
(b) choosing a preset only sets the ppem used by Create/Rasterize — it does not
**set strike metrics**; (c) a non-integer ppem is accepted silently instead of
being **rejected with the nearest valid suggestion**. Implement a kernel-side
`bitmap.grid` query that returns every exact divisor in the usable ppem range
with its grid-aligned ascent/descent, plus the exactness verdict and nearest
exact size for a requested ppem, and drive the control from it.

### Row 71 — BDF import/export with a full properties editor; PNG sheets

Works: BDF 2.1/2.2/2.3 read and 2.1/2.2 write with typed properties
(`BdfValue::{Int, Str}`); the editor adds and removes properties and marks a
value as integer; PNG sheet export/import is atomic (`SetGlyphs`), bounded and
pixel-exact in 20 frontend tests.
Missing: (a) no **rename** of a custom property and no in-place value editing;
(b) **no validation in the kernel** — a property name with whitespace or control
characters is accepted by the model and only fails at BDF export; (c) no typed
awareness of the standard XLFD properties, so the editor cannot tell the user
that `RESOLUTION_X` is an integer; (d) the PNG sheet carries **no layout
description**: the user must remember cells, columns and names, and a wrong guess
is only caught as a dimension error; (e) **`BitmapCommand::SetGlyphs` bypasses
the glyph lock** — `check_bitmap_locks` has arms for `SetGlyph`, `SetStrike` and
`RemoveStrike` only, and the `SetGlyphs` arm of `fc-doc`'s bitmap `apply` calls
the strike arm directly, below the preflight. A locked glyph can therefore be
overwritten by a sheet import. Fix all five; (e) is a defect, not a gap.

### Row 101 — multi-strike waterfall and cross-strike copy/scale

Works: `bitmap.preview` renders one row per strike using each strike's own
pixels and advances; `bitmap.transfer` copies flattened pixels between strikes
with optional nearest-neighbour rescaling of pixels, origins and metrics, in one
undo step, with a pixel-exact test.
Missing: (a) the waterfall has **no single-glyph mode**; (b) scaling is
**nearest only**, the row asks for nearest *and* area-average.

### Row 102 — EBDT/EBLC, sbix, CBDT/CBLC

Works: P3 accepted editable embedding for all three with FontTools decoding, and
`crates/fc-api/tests/bitmap.rs` proves export/reopen without stale strikes.
Missing: nothing known; the row needs re-verification against the **current**
writer with FontTools, FreeType and HarfBuzz, including a deleted-last-strike
export and the colour flavours.

### Row 103 — pixel-font auto-detection and strike reconstruction

Works: `fc_raster::detect_pixel_font` (conservative: ppem 4–256, exact common
orthogonal grid, at least three distinct staircase signatures, no curves,
effects or smart layers) and `reconstruct_pixel_font`, wired to
`bitmap.detect_pixel_grid` / `bitmap.reconstruct` and offered when the bitmap
workspace opens; 8 focused tests pass.
Missing: verification on a **real pixel font built and imported as TTF** and on a
**normal font** (DejaVu/Noto) to prove no false positive. No implementation gap
expected.

### Decisions recorded here rather than asked

1. The lock bypass in `SetGlyphs` is fixed by adding one match arm to
   `check_bitmap_locks` in `crates/fc-doc/src/command/editor_metadata.rs`. That
   file is not in my ownership list, but no wave-2 task owns locking, the change
   is a single additive arm, and putting the check anywhere else would duplicate
   the message and the policy.
2. Pixel-tool shortcuts are plain single keys gated by `when()` on the bitmap
   editor being mounted, so they cannot steal keys from the outline editor or
   from text fields. They are registered as commands so the Bitmap menu, the
   command palette and the keymap editor all see them.
3. The grid-preset range is ppem 4–512 restricted to exact divisors of the UPM;
   4 is the smallest size `detect_pixel_font` accepts and 512 is the largest
   bitmap dimension the editor allocates for a strike by default.
4. The PNG sheet layout description is carried in a `tEXt` chunk
   (`FontCarver`) written into the exported PNG and read back on import, and is
   also shown as copyable text. A sheet without the chunk still imports with
   manual settings, so nothing that worked before stops working.
5. Area-average rescaling lands in `fc-raster` (`scale_bitmap`) rather than in
   the API handler so that both filters have kernel-level tests.
