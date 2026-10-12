# Bitmap save and vertical proof follow-up

Date: 2026-10-02. Rows 71, 101 and 111. Task:
[scope and gap analysis](superpowers/plans/tasks/p5f-w3-bitmap-fixes.md).

PNG preparation now exposes **Save PNG…** through the existing `saveArtifact`
native dialog/filesystem or browser download path. Cancellation and failed
writes retain the prepared preview, embedded layout and retry button.

The bitmap waterfall has **Vertical writing**. It places upright bitmap ink
relative to both stored origin coordinates and advances top to bottom. Explicit
strike metrics win; missing vertical metrics resolve from the corresponding
glyph layer of the default master, then its VORG/default vertical metrics,
ascender and one-em fallback. Values round to strike pixels. This resolves a
cloned preview, leaving the saved font and undo history untouched. Zero advances,
negative bearings and negative advances remain valid. A visible warning reports
clipping at 16,384 pixels per side or four million pixels total. Existing bitmap
proofing still does not shape OpenType substitutions or rotate punctuation.

## Focused validation

Commands used the absolute task-local `TMPDIR`; Cargo was serialized through
`flock .scratch/p5f/cargo.lock`. No new dependencies, generated types, FontForge
code or changes to `saveArtifact` were needed.

```text
Before implementation:
  BitmapSheet: 3 existing tests passed; 3 native-save regressions failed.
  bitmapWaterfallLayout: new helper absent, suite failed to load.
  waterfall_resolves_vertical_metrics_without_changing_stored_bitmaps:
    1 failed (missing origin was null instead of inherited pixel metrics).

pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/bitmap/BitmapSheet.test.tsx \
  src/views/bitmap/BitmapWaterfall.test.tsx \
  src/views/bitmap/bitmapWaterfallLayout.test.ts
  Test Files 3 passed (3); Tests 15 passed (15).

cargo test -p fc-api --test bitmap_parity --test bitmap
  bitmap: 4 passed; bitmap_parity: 20 passed; 0 failed.
cargo clippy -p fc-api --all-targets -- -D warnings
  Finished dev profile in 20.71s; no warnings.
rustfmt --edition 2021 crates/fc-api/src/handlers/bitmap.rs \
  crates/fc-api/tests/bitmap_parity.rs
  Passed.
pnpm --dir app exec eslint <six owned bitmap TS/TSX files>
  Passed.
pnpm --dir app exec prettier --check <six owned bitmap files> src/i18n/en.json
  All matched files use Prettier code style!
```

The first whole-app `pnpm check` encountered unrelated concurrent edits:
`doc.edit_cancel` had not yet been generated in API types, and a CJK transform
test needed an `unknown` cast. After API regeneration, the last check still
reported only the CJK test cast (`CjkTransformGrid.test.tsx:377`). Final
integration reruns the whole-app gate.

The native tests use the real `saveArtifact` and native-dialog selection path,
with mocked Tauri dialog/filesystem plugin calls. They check chosen path and
exact bytes, cancellation without writing, retained preview after disk failure
and successful retry. They do not claim a native GUI session.

## Browser and review evidence

The E2E owner ran these real-kernel browser regressions with Playwright:

```text
pnpm --dir app exec playwright test e2e/bitmap.spec.ts e2e/text.spec.ts \
  --grep 'sprite sheet round trips|upright bitmap|both sidebearings'
  PNG Save PNG… download and pixel-exact import: passed (5.2s).
  Vertical bitmap canvas: passed (2.1s).
```

The vertical browser fixture paints a 10 × 44 canvas with two 3 × 4 ink blocks
at `(2, 15)` and `(0, 29)`, 24 ink pixels total. Those browser cases used the
existing kernel's explicit strike-origin fields; inherited metrics are covered
by the new API regression above and the parent rebuilds the kernel before the
final integrated suite. See the [E2E acceptance](p5f-w3-e2e-acceptance.md) for
the exact browser command, environment and final run. No browser failure was
hidden with a skipped PNG assertion or a changed download implementation.
Independent source review found no blocking defect and repeated the 15 focused
bitmap tests successfully.

## Localization and completion

English changes handed to the parent's independent GPT-6.1 Sol translation
pass: changed `bitmapSheet.download`; added `bitmapSheet.saved`,
`bitmapSheet.errors.save`, `bitmap.waterfallVertical`,
`bitmap.waterfallVerticalHint`, `bitmap.waterfallCanvas` and
`bitmap.waterfallClipped`. This feature author did not edit Chinese.

Rows 71/101/111 retain their existing accepted format/shaping boundaries. These
ordinary native-saving and vertical-bitmap-proof gaps are verified and are
recommended **Complete**, subject to the parent's integrated localization and
full-suite gate. Parent owns the audit status.

Final resource check: build 15.88 GiB / 19,638 files; shared temporary outputs
0.01 GiB / 612 files; 606.07 GiB free; **PASS**, with the existing 10 GiB build
warning. No browser/server processes were started by this task. All owned test
processes finished and `.scratch/p5f-w3-bitmap-fixes` was removed.
