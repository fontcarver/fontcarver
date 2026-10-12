# Bitmap parity integration acceptance — 2026-09-30

The resumed bitmap work adds cross-strike copy/nearest scaling and depth conversion, atomic sheet glyph replacement, editable outline pixel components, four trace treatments with independent diagonal joins, exact integer-UPM grid presets, source/resolved bitmap reference separation, reference editing, PNG sheets, canvas eyedropper/panning/guides/HUD and conservative pixel-font reconstruction.

## Integrated paths

`BitmapView` mounts the new controls; trace/raster remain existing routes. New API methods are `bitmap.transfer`, `bitmap.pixel_components`, `bitmap.detect_pixel_grid`, and `bitmap.reconstruct`. Sheet import uses `BitmapCommand::SetGlyphs`, preserving other strike images and producing one undo step. `bitmap.trace` accepts optional `mode` and `join_diagonals`, keeping the prior exact default. No generated API files were hand-edited.

The reusable outline `pixel` glyph is a UPM-square by default; components scale it by 1/ppem. Its existing outline is reused. The bitmap-reference editor preserves raw images while displaying resolved images; local crop/scale/transform operations retain references, with explicit flattening available. PNG import is bounded before browser decode and rejects clipped/nonempty padding cells.

## Verification actually run before root integration

- References/session/grid: 3 Vitest files, 13 tests passed; owned ESLint passed.
- PNG sheet/codecs/image decode: 3 Vitest files, 20 tests passed; owned ESLint passed.
- Canvas/render/geometry/pixels: 4 Vitest files, 14 tests passed; a subsequently added reference-preservation regression passed in the 6-test pixel suite. Owned ESLint passed.
- Integration controls/transforms: `pnpm --dir app exec vitest run src/views/bitmap/BitmapParity.test.tsx`: 4 tests passed. Owned integration-file ESLint passed.
- `git diff --check` and owned Rust formatting checks passed.
- API test attempt stopped at unrelated shared `fc-sfnt`/`fc-formats` compilation failures before bitmap tests executed. Trace/reconstruction queued runs were cancelled by parent coordination. A duplicate child global TypeScript check was interrupted; another failed on stale generated APIs. Neither is a passing integration check.

Root owns final API generation, workspace Rust checks/tests, consolidated frontend checks and browser verification. No browser acceptance is claimed in this report. Existing bitmap e2e pixel selection was updated for the new one-pixel guide margin.

## Pending integration evidence

Run `fc-raster` tests `trace_modes`, `pixel_reconstruction`, and `raster`, plus `fc-api` tests `bitmap_parity` and `bitmap`. New API tests cover transfer pixels/metrics/depth, stale writes, atomic failures and sheet batches, pixel-component undo/persistence, and TTF/OTF import/detect/reconstruct/undo using exact independent bitmap masks. Trace tests exhaust 512 small masks across corner/join policies.

Browser acceptance should verify the assembled editor, PNG import/export, trace modes, component editing, cross-strike transfer and reconstructed outlines. Root's accepted policy freezes new scope after finishing this work.

## Boundaries

Pixel detection is deliberately conservative: ppem 4–256; at least three distinct staircase shapes; all master outlines must lie on the exact common orthogonal grid. Curves, approximate grids, effects and smart outlines are rejected. Detection runs when the bitmap workspace opens and reconstruction is an explicit undoable operation; source import itself remains unchanged. Transfer/reconstruction batches cap at 4096 glyphs; pixel-component generation caps at 16,384 components per glyph and 100,000 per transaction. PNG sheets cap at 256 glyphs. No unsupported platforms or unexecuted tests are marked complete.

No new dependency was added by this task. The concurrent source-image owner separately added `png` to `fc-raster`. No FontForge implementation was read or adapted.

Final local resource check: build 6.09 GiB / 13,176 files; all task temporary artifacts removed; workspace-wide temporary 0.22 GiB / 7,912 files; 668.66 GiB free; PASS. No owned processes remain.
