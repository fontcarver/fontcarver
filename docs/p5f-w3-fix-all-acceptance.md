# Outline audit Fix All and atomic cancellation acceptance

2026-10-02. Audit row 35: **Complete** for the four existing safe checks.

Fix All now re-audits current kernel geometry between bounded repair passes in
one editing gesture. A contour with an open seam, duplicate nodes, fractional
coordinates and two missing cubic extrema is repaired with one press and one
undo entry. Descending curve splits rescale each subsequent parameter against
the remaining left segment. Single Fix retains its reviewed-issue scope.
Disabled checks cannot consume the fresh validation report's 100-marker cap.
Locked contours remain unchanged; the existing kernel rejects glyph/path locks,
read-only documents and invalid commands.

A new token-checked `doc.edit_cancel` restores only its matching live gesture.
It preserves the previous undo/redo branch and dirty state and emits one event
with the final history labels. It never undoes a newer gesture. The shared
`EditSession.cancel` uses this endpoint, waits for in-flight applies and does
nothing after the session has ended. Macro and Python callers cannot invoke
this editing endpoint directly.

## Validation

All Cargo commands used `.scratch/p5f/cargo.lock`; tests used absolute task
TMPDIR. Generated API types were regenerated with the existing generator.

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/glyph/outlineAudit src/editor/editSession.test.ts \
  src/automation/runtime.test.ts
6 files passed; 69 tests passed

cargo test -p fc-doc --test edit_cancel
5 passed; 0 failed; 0 ignored
cargo test -p fc-api --test edit_cancel -- --nocapture
1 passed; 0 failed; 0 ignored (independent reviewer)

pnpm --dir app gen:api
Finished dev profile; generator succeeded
pnpm --dir app check
PASS (tsc -b)
pnpm --dir app exec eslint <owned frontend files and E2E spec>
PASS
pnpm --dir app exec prettier --write <owned frontend files>
PASS
rustfmt --edition 2021 <owned Rust files>
PASS
```

The initial multiple-extrema regression failed (1 failed / 21 passed): the
planner emitted only one split. The nonempty cancellation/redo regression also
failed before the history baseline fix: cancelling had erased the earlier redo
branch. Both now pass. The integrator's Clippy pass found one collapsible nested
condition; it was collapsed and the owned Rust file reformatted. Broad workspace
checks belong to the integration report.

Real-kernel browser evidence:

```text
TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5f-w3-fix-all/tmp \
FC_API_PORT=8853 FC_UI_PORT=1463 \
pnpm --dir app exec playwright test e2e/outline-audit.spec.ts \
  --output=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5f-w3-fix-all/results \
  --reporter=line
1 passed (5.6s)
```

The test creates a small source font, opens its glyph from the grid, presses the
visible Fix All control and checks the stored contour. All four defect types
are resolved, both extrema become on-curve nodes, a locked contour is unchanged,
history increases by exactly one entry, undo and redo restore exact outlines,
and a saved/reopened `.fcf` retains the result. It uses the release kernel built
at 15:44; the later cancellation-history fix is independently verified by the
five document tests and subscriber assertion above. Two earlier browser attempts
failed in fixture setup (missing reload after API creation, then an exact tab
name omitting the Overview prefix), before repair execution; these were test
setup corrections, not changed product assertions.

## Review round

Independent review: [kernel review](p5f-w3-review-kernel.md). Corrected findings:

- Disabled checks filling the fresh marker cap: revalidate requested check kinds;
  a 110-fractional-point / two-extrema regression covers it.
- Nonempty cancellation losing prior redo: retain a structurally shared history
  baseline before the first applied gesture command.
- Cancellation publishing an obsolete redo label: install restored history
  before broadcasting; a subscriber asserts exactly one consistent event.

Existing Escape/drag/metrics callers were reviewed separately under
[p5f-w3-cancel-callers-acceptance.md](p5f-w3-cancel-callers-acceptance.md).

## Limits and ownership

Fix All stops after 32 passes, beyond 100,000 points, or when an outline repeats;
its matching live gesture is cancelled and the panel requests individual review.
A superseded token reports the kernel error without an unscoped undo fallback.
Ambiguous coincidences across contours or involving handles remain manual.
No new geometry approximation, model/schema field, dependency or FontForge code
was introduced. Native desktop input was not automated.

Shared changes: document gesture lifecycle; API method/handler/generator and
script guards; `EditSession`, mock transport and focused tests; outline audit
planner/helper/panel and tests; one E2E spec; the outline-editor user guide.
English changes are `editor.audit.pass`, `editor.audit.truncated` and new
`editor.audit.limit`; the separate GPT-6.1 Sol agent translated/reviewed them.

Owned fixture kernels and Vite exited through Playwright teardown. Task scratch
and generated fonts are removed after retaining this evidence. Final resource
check is recorded below.

```text
python3 scripts/check_artifacts.py
build: 2.19 GiB, 4,515 files
temporary: 0.02 GiB, 1,770 files
workspace free: 619.73 GiB
PASS
```

The task began at 15.45 GiB of build artifacts and reached 15.88 GiB before
integration. Its final cleanup initially observed the concurrent integration
build at 28.31 GiB (FAIL). The parent stopped that build, recorded it as
interrupted and removed regenerable development-profile artifacts under the
shared lock. The checkpoint above passed after that coordinated cleanup;
release caches were retained. This task's own scratch and processes are gone.
