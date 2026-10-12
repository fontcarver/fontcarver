# Independent outline pointer lifecycle review

2026-10-02. Implementation owner: `w3_review_editing`. This reviewer changed no
product code. Scope: [review task](superpowers/plans/tasks/p5ux-outline-review.md).

**Verdict: approved after one corrected finding.** No remaining source blockers
were found for the scoped pointer lifecycle repair. Actual browser walkthrough
and broad integration checks remain the parent's responsibility.

## Finding and fix round

**P2 — releasing a secondary mouse button last stranded a primary drag.**
During a left-button drag, hold right, release left first, then release right.
Pointer Events report the last release with `button = 2`. The canvas helper
cleared its ownership, but the controller rejected that pointer-up, leaving the
drag token open. Subsequent hover could continue editing without a held button.

An independent helper/controller probe reproduced this: three pending-apply
cancellation probes passed, while the chord probe failed because neither
`doc.edit_end` nor `doc.edit_cancel` occurred. The implementation owner separately
reproduced the same defect and corrected it:

- The helper remembers the initiating button mask (primary 1, middle 4).
- A move without that held button cancels the owned gesture immediately.
- The controller accepts the final release of a canvas-owned pointer even when
  another physical button happens to be released last.

Both chord paths now pass. Once cancellation finishes, later hover produces no
additional editing calls. Regressions are retained in the pointer helper and
controller test files.

## Reviewed behavior

- Secondary and non-primary pointer-down events do not capture, select or edit.
  Primary drawing and middle-button panning remain available.
- Pointer IDs isolate the active gesture. Normal release clears ownership before
  releasing capture, so the resulting lost-capture event does not cancel a
  committed edit.
- Pointer cancellation, lost capture, canvas/window blur, hidden document and
  Escape during a captured gesture enter cancellation rather than successful
  pointer-up. Space is cleared without generating a fill-toggle tap.
- Node, metrics and transform cancellation reaches the token-scoped
  `doc.edit_cancel`. The three independent delayed-apply probes observed the
  sequence `begin → apply → cancel → next begin → next apply → end`, with no
  transport error or unscoped `doc.undo`.
- The existing atomic kernel endpoint restores earlier undo/redo and dirty
  state; that implementation and subscriber evidence were independently
  accepted in [the kernel review](p5f-w3-review-kernel.md). No kernel code changed
  in this pointer repair, and no Cargo checks were rerun for this review.
- Cancelling an active Pen press discards its unfinished local draft. Blur
  between ordinary Pen clicks preserves the draft, as does an interrupted
  temporary pan between clicks. Normal multiple-click drawing, finish/tool
  switching and Space tap versus Space used during a press retain focused
  test coverage.

## Independent verification

All commands used an absolute reviewer TMPDIR under
`.scratch/p5ux-outline-review/tmp` and one Vitest worker.

```text
pnpm --dir app exec vitest run \
  --config ../.scratch/p5ux-outline-review/vitest.config.mjs \
  --maxWorkers=1 --minWorkers=1
Initial probes: 3 passed; 1 failed (secondary final release)
Final probes:   6 passed; 0 failed (2.53s)

pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/editor/editorController.test.ts \
  src/views/glyph/canvasPointerSession.test.ts \
  src/editor/tools/transformTool.test.ts src/editor/tools/pen.test.ts
4 files passed; 128 tests passed (5.18s)
```

The six reviewer probes cover delayed node/metrics/rotation applies, final
secondary-button release, cancellation when the initiating button disappears,
inert hover after cancellation, and document visibility interruption clearing
Space without toggling fill. They used temporary copies of existing fixture
builders and the real controller/session implementation; no product test file
was edited by the reviewer.

`git diff --check` passed for the reviewed tracked source changes. No new strings,
dependencies, browser/server processes, native input automation or broad suites
were introduced by this review. Reviewer scratch was removed after recording
the evidence above; the final resource checkpoint follows.

```text
python3 scripts/check_artifacts.py
build: 5.33 GiB, 12,257 files
temporary: 0.13 GiB, 5,026 files
workspace free: 616.32 GiB
PASS
```
