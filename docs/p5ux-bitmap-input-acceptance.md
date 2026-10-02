# Bitmap gesture cancellation and temporary navigation

2026-10-02. Scope: [task](superpowers/plans/tasks/p5ux-bitmap-input.md).

The bitmap canvas now cancels an active stroke or selection drag on Escape,
pointer cancellation, unexpected capture loss, canvas focus loss or window
blur. It restores the starting pixels and selection. A session checkpoint also
restores the earlier dirty state, composed preview and write error, so a later
Save neither writes a cancelled stroke nor discards an earlier failed draft.
Normal pointer-up commits once; subsequent capture loss is harmless.

Middle-button drag and Space + left drag temporarily pan with any chosen tool,
including while pixel editing is disabled. Releasing Space or losing focus
restores the chosen tool. Arrow keys also pan while Space is held. Existing
Hand/eyedropper behavior is preserved; right-click had no bitmap tool binding.

Independent review identified two further races and the fix covers both:

- A save/navigation barrier now synchronously accepts and ends the canvas
  gesture before writing. Motion or Escape from that old held pointer cannot
  create another dirty draft. The accepted selection remains in place.
- Reload is deferred while a gesture checkpoint exists, including a clean
  marquee. Completing or cancelling refreshes afterward. Cancellation cannot
  strand `busy` or pair stale pixels with a newer generation token.

## Validation

Focused tests used the absolute task-local `TMPDIR`. The initial regressions
failed as expected: **9 failed / 12 passed**. Final command:

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/bitmap/BitmapCanvas.test.tsx \
  src/views/bitmap/bitmapSession.test.ts
  BitmapCanvas: 19 passed; bitmapSession: 8 passed.
  Test Files 2 passed (2); Tests 27 passed (27).

pnpm --dir app exec eslint src/views/bitmap/BitmapCanvas.tsx \
  src/views/bitmap/BitmapCanvas.test.tsx src/views/bitmap/BitmapEditor.tsx \
  src/views/bitmap/bitmapSession.ts src/views/bitmap/bitmapSession.test.ts
  Passed.
pnpm --dir app check
  tsc -b: passed.
pnpm --dir app exec prettier --check <the five owned bitmap files>
  All matched files use Prettier code style!
```

The chained canvas/session test moves selected pixels, flushes while the pointer
is still held, then moves again, presses Escape and releases. It proves one
kernel write, no resumed draft, and retained accepted selection. Independent
review separately reproduced and rechecked the asynchronous reload races.

The first whole-app type check encountered unrelated in-progress outline and
shortcut test edits (`cancelPointer`, `canvasPointerSession`, shortcut function
signature). After those edits settled, the final whole-app check passed.

The E2E owner also verified the live browser/kernel:

- Escape, pointer cancellation and blur produce no `bitmap.apply`; saving and
  reopening retains the original pixels (**passed, 3.6 s**).
- On a 64 × 64 bitmap at 32× zoom, middle drag and Space + left drag move the
  actual scroll position from `(100, 100)` to `(160, 150)`, retaining pencil
  selection and every pixel (**passed, 2.1 s**).

The independent [bitmap review](p5ux-bitmap-review.md) approved the source and
reran all 27 tests successfully, including the save-barrier regression.

No backend or dependency changes. The feature author changed no locale files;
the parent owns any discoverability wording and its separate translation pass.

Final resource check: build **5.33 GiB / 12,257 files**, shared temporary output
**0.12 GiB / 4,556 files**, free **616.32 GiB**, **PASS**. This task started no
browser/server processes; its test processes finished and its scratch directory
was removed. Other agents own the remaining shared temporary output.
