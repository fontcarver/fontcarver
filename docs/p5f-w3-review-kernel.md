# Independent kernel review: real hinter and atomic repairs

Reviewer scope: real `ttfautohint` output validation and independent font
consumption, followed by token-checked edit cancellation and bounded Fix All.
No implementation files are owned or edited by this review.

## Real hinter verification

The authentic Ubuntu-extracted `ttfautohint 1.8.4` executes through the existing
CLI. The reviewer independently ran:

```sh
PATH="$PWD/.scratch/p5f-w3-hinter/root/usr/bin:$PATH" \
LD_LIBRARY_PATH="$PWD/.scratch/p5f-w3-hinter/root/usr/lib/x86_64-linux-gnu" \
TMPDIR="$PWD/.scratch/p5f-w3-review-kernel/tmp" \
python3 scripts/p5f_real_hinter_acceptance.py \
  --output .scratch/p5f-w3-review-kernel/real
```

Result: six outputs, 5,184 successful FreeType glyph renders, 18 matching
HarfBuzz shaping comparisons, and three invalid-input cases preserving an
existing destination. Default TTF/WOFF/WOFF2 raster hashes match. Composite
hinting appends the documented unmapped helper; subglyph adjustment and dehinting
retain the original glyph count. Original glyph names, cmap and metrics match.

## Findings

1. **P2 — newly allowed helper count accepted a malformed output.** The initial
   composite exception accepted any output whose `maxp.numGlyphs` increased by
   one. A reviewer-controlled executable copied its input, incremented only that
   count, and returned success. `fontcarver build … --autohint --no-preserve
--hint-composites` exited successfully and wrote a corrupt font:
   `maxp=145`, `loca entries=145` (146 required). FontTools reported
   `corrupt 'loca' table, or wrong numGlyphs in 'maxp': 144 145`.
   **Closed after correction.** The rebuilt CLI rejects that malformed output
   (`An offset was out of bounds`) and rejects a structurally valid authentic
   helper exposed at U+0041 through cmap (`output glyph count changed`). Both
   failures preserve an existing destination. The final authentic composite
   output still passes 864 FreeType renders and three HarfBuzz comparisons.

2. **P2 — disabled checks prevented an offered repair.** With extrema alone
   enabled, 110 fractional points in another contour consumed the planner's
   independent all-checks marker cap. The visible report offered two extrema
   fixes but the planner returned `null`. **Closed after correction:** stale
   revalidation now uses the requested check kinds; the reviewer reran the same
   fixture and received both split commands. A regression is retained.

3. **P2 — a cancelled nonempty repair erased prior redo history.** Starting a
   repair after Undo discards the existing redo tail at its first applied
   command. Restoring geometry and dropping the failed repair's redo cannot
   reconstruct that unrelated history. **Closed after correction:** the active
   gesture now retains a structurally shared history baseline. Cancellation restores it before broadcasting; the
   regression proves the previous redo branch and saved dirty state survive.

4. **P2 — cancellation broadcast exposed an obsolete redo label.** Calling
   `undo()` emitted a `DocumentEvent` before the cancelled redo entry was removed.
   The frontend copies `event.redo_label`, leaving a visible Redo Repair action
   that the kernel cannot perform. **Closed after correction:** cancellation
   installs the saved history before restoring the entry. The reviewer-run subscriber regression receives exactly
   one event with the original redo label and correct dirty state.

## Atomic cancellation review

The new dispatch method uses `Session::with_doc_mut` for the whole token check,
rollback and cancelled redo-entry discard. A stale token returns an error before
clearing any active gesture. The frontend now uses this endpoint and never falls
back to an unscoped global undo; cancelling an already ended session is a no-op.
The endpoint is excluded from the generated macro surface and the runtime
JavaScript/Python allowlists.

Reviewer commands, both under the shared Cargo flock and reviewer TMPDIR:

```text
cargo test -p fc-doc --test edit_cancel -- --nocapture
5 passed; 0 failed; 0 ignored
cargo test -p fc-api --test edit_cancel -- --nocapture
1 passed; 0 failed; 0 ignored
```

An initial run hit a transient `LayerId: From<&str>` error in the new test;
that error was corrected before the passing runs above. The added
empty-gesture test proves an existing redo tail remains available. The added
nonempty-gesture regression proves old redo history and the single emitted event
are restored consistently.

Focused frontend review after the marker-cap fix:

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/glyph/outlineAudit.test.ts \
  src/views/glyph/outlineAuditRepair.test.ts \
  src/views/glyph/OutlineAudit.test.tsx \
  src/editor/editSession.test.ts src/editor/gestures/drag.test.ts
5 files passed; 64 tests passed
```

The reviewer checked bounded re-auditing, descending/rescaled curve splits,
cycle refusal, path locks, stale snapshots and document handover ordering. The
barrier is published before the first API read; later gesture intent invalidates
the current repair before the waiting gesture acquires a token. Cancellation
failure is reported as a kernel error and never followed by a global undo.

## Final real-kernel Fix All evidence

The implementation agent's `app/e2e/outline-audit.spec.ts` passed **1/1 in 5.6 s**
against the release kernel on API port 8853 / UI port 1463. The reviewer inspected
the final spec, the retained Playwright status (`passed`, no failed tests), the
owned kernel startup log and the saved `.fcf` artifact. The test uses independent
geometry assertions: one press resolves open/fractional/duplicate defects and
both curve extrema; the locked contour stays unchanged. History gains exactly
one entry; undo/redo and save/reopen reproduce the expected contours.

The reviewer independently parsed the saved FCF JSON: the repaired contour is
closed with ten integer-coordinate points and extrema at `(87, 208)` and
`(-86, 592)`. The locked contour still starts at fractional `x=1000.2`, and its
`locked_paths` metadata remains intact. The final cancellation-history correction
is separately covered by the five Rust tests and subscriber assertion above;
the browser success path does not exercise cancellation.

See [implementation acceptance](p5f-w3-fix-all-acceptance.md) for the exact run
command. No rerun was needed because the spec and retained evidence agree.

## Reviewer resource checkpoint

Final reviewer cleanup removed `.scratch/p5f-w3-review-kernel/` (436 KiB).
All reviewer-started Vite SSR instances and test processes have exited. No shared
caches, other tasks' files or original fixtures were removed.

```text
python3 scripts/check_artifacts.py
build: 15.88 GiB, 19,640 files
temporary: 0.02 GiB, 2,108 files
workspace free: 606.04 GiB
warnings: build artifacts exceed 10 GiB; plan profile-cache cleanup
PASS
```

Verdict: approved after four corrected findings and final real-kernel acceptance.
No review gate remains open for this task.

## Evidence follow-up resource checkpoint

The final Fix All acceptance command and output were inspected after the report
was written; the real-kernel review gate is closed. This follow-up started no
builds or servers and recreated no reviewer scratch. Concurrent integration
builds increased the shared cache beyond its limit:

```text
python3 scripts/check_artifacts.py
build: 28.31 GiB, 21,402 files
temporary: 0.02 GiB, 1,770 files
workspace free: 593.62 GiB
errors: build artifacts exceed the capacity ceiling
FAIL — pause builds and inspect disposable artifacts
```

The integrator was notified and owns cleanup of its active build outputs. This
supersedes the earlier passing resource snapshot; project completion must record
a passing resource check after that cleanup. No other task's cache was removed
by the reviewer.

The integrator subsequently stopped its build, cleaned the dev profile and
reran the full workspace successfully. A second planned cleanup before native
packaging left **2.54 GiB build / 0.04 GiB temporary / 619.25 GiB free, PASS**.
The final resource closure is recorded in
[integrated acceptance](p5f-w3-integration-acceptance.md); the interrupted run
above remains an interrupted run.
