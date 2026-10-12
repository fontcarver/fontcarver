# Independent bitmap gesture review

2026-10-02. Scope: `BitmapCanvas`, `BitmapEditor`, `BitmapSession` and their
focused regressions. The reviewer changed no product source.

## Findings and corrections

1. **P2 — pointer continuation after a Save barrier escaped cancellation.**
   The original barrier cleared the session checkpoint while leaving the canvas
   gesture active. After Save completed, another move could create a dirty draft
   with no cancellation checkpoint, so Escape followed by another Save could
   persist the allegedly cancelled continuation. The corrected session invokes
   a synchronous acceptance callback, releasing the canvas gesture before the
   flush begins. Accepted pixels and the resulting selection are retained;
   later events from that pointer cannot continue the old stroke.

2. **P2 — cancelling a clean gesture raced with document refresh.** Independent
   probes using the actual session implementation reproduced both variants:
   cancelling during `load()` left `busy=true` permanently; cancelling after a
   completed refresh paired old pixels (`advance=8`) with the new generation
   (`2`, whose actual pixels had `advance=99`). The latter could overwrite newer
   data without a generation conflict. The correction defers refresh while a
   gesture checkpoint exists, then refreshes after finishing or restoring it.
   The reviewer reran both probes: loading finishes with `busy=false`, and the
   latest pixels and generation remain consistent (`advance=99`, generation 2).

The review also checked that normal pointer-up clears gesture ownership before
releasing capture, preventing a subsequent lost-capture event from undoing the
accepted stroke. Pointer cancellation, Escape and blur restore the captured
selection. The session restores original value, resolved preview, dirty flag
and error, preserving earlier unsaved edits without creating a replacement
write. Middle-button and temporary Space navigation retain the selected pixel
tool and remain available while editing is disabled.

## Validation

```text
TMPDIR=$PWD/.scratch/p5ux-bitmap-review/tmp \
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/bitmap/BitmapCanvas.test.tsx \
  src/views/bitmap/bitmapSession.test.ts
2 files passed; 27 tests passed (19 Canvas + 8 Session)
```

The reviewer additionally ran the two refresh/cancel race probes through Vite's
SSR loader, using the actual session and a deferred transport. Each Vite instance
was closed afterward. No Cargo or broad frontend checks ran.

The final combined Canvas/Session regression was independently run: a held
selection move is accepted by `session.flush()`, then pointer motion, Escape,
pointer-up and another flush produce no additional edit or write. The accepted
selection stays at its new position, the draft stays clean and exactly one
`bitmap.apply` occurs. Actual `BitmapEditor` callback wiring was checked too.

Verdict: **approved**, both findings corrected and independently verified.
Browser-wide UX acceptance and project-wide checks remain with the integrator.

## Cleanup and resources

Owned test processes and Vite SSR instances exited; reviewer scratch
`.scratch/p5ux-bitmap-review/` was removed. No other task's files were deleted.
Final `python3 scripts/check_artifacts.py`: **PASS** — build 5.33 GiB /
12,257 files; temporary 0.11 GiB / 4,040 files; free 616.31 GiB.

## Follow-up: delayed bitmap-entry focus

The full browser suite found that menu entry could leave focus on the closed
menu title after the bitmap canvas finished loading. The narrow repair adds a
mount-only focus handoff when focus is still on the body or a closed menubar
title. Existing fields, buttons and open menus retain ownership; asynchronous
loading cannot refocus an already mounted canvas on later data refreshes.

Review found an additional same-commit modal race: dialog DOM can be mounted
before its passive effect increments `modalCount`, so checking the store alone
could briefly hand focus to the canvas behind a newly rendered dialog. A real
`Dialog` regression failed before correction. The effect now also checks
`[aria-modal="true"]`; the final regression proves the canvas never receives
focus before the dialog's input does.

The reviewer checked active-tab rendering and async ownership: only the current
tab mounts `BitmapView`; a detached/unmounted loading result cannot mount its old
canvas. No timer or subscription retries focus later. A focused closed title
also remains untouched when hover opens another menu. The effect changes no
pointer or gesture lifecycle code.

Independent final command:

```text
TMPDIR=$PWD/.scratch/p5ux-bitmap-review-focus/tmp \
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/bitmap/BitmapEntryFocus.test.tsx \
  src/views/bitmap/BitmapCanvas.test.tsx
2 files passed; 28 tests passed (9 entry focus + 19 canvas gesture)
```

These tests cover delayed pointer/keyboard menu entry, all nine pixel-tool keys,
initial body focus, preserved field/button/open-menu/modal ownership, the
same-commit modal race and existing cancellation/navigation behavior.
**Follow-up approved.** Integrated browser verification remains with the owner.

Follow-up tests exited and `.scratch/p5ux-bitmap-review-focus/` was removed.
Final resource check: **PASS** — build 5.33 GiB / 12,257 files; temporary
0.06 GiB / 690 files; free 616.39 GiB. No Cargo or full-suite run was started.
