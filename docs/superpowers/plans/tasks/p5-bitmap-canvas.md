# P5 bitmap canvas tools and guides

## Scope and ownership

Own bitmap canvas, canvas controls/styles, pixel gesture helpers and tests, bitmap
renderer and focused tests. Parent integrates `BitmapEditor` and `BitmapView`.
No generated API edits, Rust changes, commits, or translations.

## Public interface

`PixelTool` adds `eyedropper` and `hand`. Pencil gestures determine ink from the
first pixel (matching ink clears, otherwise selected ink), then preserve that ink
throughout the drag. Canvas accepts optional `upm`, `ppem`, `outlinePath`,
`resolved` bitmap, and `onPickInk`. Controls expose grid and outline overlays,
advance guides, and a pointer/delta HUD with units per pixel. Hand drag pans the
scrolling canvas without changing pixels. Eyedropper reads packed coverage/RGBA
without issuing edit commands. Font coordinates remain y-up.

## Acceptance

Write failing tests for toggle-and-match, passive tools, point conversion and
renderer guide alignment. Verify no edits/commits for eyedropper or hand,
consistent drag ink, retained baseline/advance guides when grid is hidden,
origin-aware outline transform, and pointer/delta readouts. Run focused tests,
then app check/lint/test through parent-coordinated broad checks. Parent owns
integrated headless agent-browser visual acceptance.

## Resource handling

Use `.scratch/p5-bitmap-canvas/` with an absolute TMPDIR, check artifacts at task
start and completion, and remove only owned scratch. No additional dependencies.

## Acceptance record

- Red: initial pixel tests failed for absent `pencilInk` and passive tools
  changing pixels; geometry tests failed for missing coordinate helpers.
- Green: `pnpm --dir app test src/views/bitmap/pixels.test.ts
  src/canvas/bitmapGeometry.test.ts src/canvas/bitmapRenderer.test.ts
  src/views/bitmap/BitmapCanvas.test.tsx --maxWorkers=1`:
  **4 test files passed, 14 tests passed** (9.99 s).
- Owned-files ESLint: exit 0; owned tracked-file `git diff --check`: exit 0.
- Final resource check: **PASS**; build 6.07 GiB / 13,160 files, temporary
  0.03 GiB / 2,605 files, workspace free 669.20 GiB. No owned scratch,
  browser, or server processes were created.
- Parent owns `BitmapEditor`/`BitmapView` integration and final broad app
  validation plus headless visual acceptance. No FontForge source was read.
- Integration regression: pixel-plane transforms previously cleared refs. New
  test failed first, then `transform()` was changed to retain source references.
  Focused rerun: **1 file passed, 6 tests passed** (2.52 s); other canvas tests
  were unaffected. Parent applies the same policy to crop/resample.
- A broad `pnpm --dir app check` had already started when parent requested
  avoiding duplicate checks. It was interrupted (exit 130), after reporting
  current generated-API mismatches in other concurrent work. It is **not a
  passing check**; parent/root own regeneration and final validation.
