# P1 acceptance

Status: **P1 implementation and headless acceptance complete; native interaction
paused**, 2026-09-07. Final production candidate `2e53ba6` passed whole-branch
review and scoped re-review. The user requested a stop after P1; no P2 work or
merge to `main` was performed. Remaining native interaction and release packaging
are explicitly unverified below.

## Kernel and CLI workflows

`crates/fc-api/tests/acceptance_workflow.rs` opens the licensed DejaVu Sans
fixture, edits a point in each of A, B and C, exports TTF, reparses the result
and checks the changed coordinates. It verifies byte-identical GSUB/GPOS/GDEF
tables where present and an unchanged shaping fingerprint for `Hello fi AVWA`.
The three directly edited glyphs invalidate component dependents too; the
export report is checked against the actual document invalidation count.
DejaVu is the redistributable representative real font used for this workflow.

`crates/fc-cli/tests/acceptance_workflow.rs` creates Scratch Latin through the
API, adds A–C, draws outlines, sets widths to 600/620/640, exports TTF and invokes
the current Cargo-built CLI to inspect, list and shape it. Reopening the SFNT
verifies A's width and outline. Both workflows passed in the implementer's
isolated worktree. Final integrated pointer-driven browser counterparts also
passed: DejaVu in 3.3 s and Scratch Latin in 3.5 s.

## Preservation and shaping

Nine `fc-compile` preservation tests passed on the final Rust candidate. Entirely untouched fonts with
verified original glyph set/order and an unambiguous original face retain
`glyf`, `loca`, `maxp`, global hint programs and glyph instructions verbatim. **Any outline edit drops all
TrueType hinting and emits a warning**, including instructions in otherwise
untouched glyphs. This is the explicit Task 3 fix-1 C1 ruling, a limitation of
P1 relative to the original broader preservation wording. Edited-font tests
verify both global and per-glyph instruction removal. When multiple TTC faces
have the same glyph order, original-face preservation is declined: export uses
the selected face's model geometry and drops hinting with the existing warning.
A generated two-face DejaVu regression verifies that face 1 cannot silently
export face 0's outlines. All nine preservation tests passed after this fix.

The Noto shaping round trip passed. Compiled model glyph names are retained
before large-font `post` format 3 removes font-file names; a focused regression
shapes `fi` into an unencoded `f_i` and verifies its model identity. Real Source
Han import also passed the Adobe/Identity CID and `uni4E00` identity regression.

## Performance evidence

Measurements below were taken on this development machine while other worktrees
were active, rather than in an isolated benchmark environment: Intel Core
i7-12700H, 20 logical CPUs, approximately 62 GiB RAM, Linux, Rust 1.94.0.

| Gate | Observed | Budget | Status |
| --- | --- | --- | --- |
| Rust API `doc.edit_apply`, release | median 0.556 ms; p95 0.602 ms; max 1.411 ms | p95 ≤ 4 ms | Passed |
| DejaVu Sans TTF export, release | 0.582 s | ≤ 5 s | Passed |
| Source Han Sans open, release | 0.900 s | < 3 s | Passed |
| Source Han Sans export, release | 58.697 s | ≤ 60 s | Passed |
| Pointer handler through synchronous repaint, 300 points | median 0.900 ms; p95 1.500 ms; max 5.200 ms | every sample ≤ 8 ms | Final passed |

The API gate uses exactly 20,000 populated glyphs: one 300-point target and
19,999 four-point outlines with widths and master layers. It performs 20 warmup
and 200 measured calls. Timing includes API dispatch, request decoding, document
locking, actual mutation, coalescing and response construction; it excludes
caller JSON construction, frontend transport and rendering. The test checks
cumulative movement and a single undo restoring the original coordinates.
Run it explicitly with `cargo test --release -p fc-api --test edit_performance
-- --ignored --nocapture`; a debug run cannot satisfy this release-only gate.

The browser gate sends one pointer move per animation frame, with 20 warmup
and 200 measured moves. Its timer covers event dispatch through synchronous
canvas drawing and checks that every move repaints and changes the preview;
frame waiting is outside that timer. It verifies persisted coordinates and a
single undo. Pointer-down measured 2.400 ms, and both pointer-down and every
measured move must meet the 8 ms budget. These are CPU drawing measurements,
not a measurement of display scanout. An earlier synthetic burst of all moves
in one JavaScript turn produced 13–14.4 ms spikes; that diagnostic is retained
in the browser report. Renderer-only `__fwPaint` timings are separate.

Initial real Source Han `text.shape` took 76.779 seconds. The HTTP transport
now allows 120 seconds for shaping and export, retaining 30 seconds for other
methods and honoring explicit caller overrides. No automatic retry was added.

Source Han used the actual 16,529,832-byte external OTF, resolved through the
portable fixture lookup in a fresh linked worktree. The export gate verified
65,535 glyphs and the U+4E00 cmap entry. Neither measurement early-returned.
Export passed with only 1.303 seconds of headroom; this is a narrow margin on
this machine, not a guarantee for every development environment.

## UFO, UI and desktop

Eight `fc-formats` UFO write tests passed, including semantic, incremental and
SFNT/UFO round trips. Final browser acceptance passed three tests:
DejaVu pointer edits to A/B/C with export/reopen; new-font pen drawing and
600/620/640 widths with export, release CLI inspection and reopen; and the
300-point drag gate. T8 and independently checked Chinese translations are
merged. T9 TextView integration now covers shaped runs, glyph-local editing,
coalesced sidebearing keys, stable tab identity and delayed master handover.
The final independently translated/reviewed Chinese locale has 881 matching
keys. The final integrated app passed **104 files / 930 tests** in 10.76 s.
The fresh release-kernel headless suite passed **21 tests, no skips**, in 40.9 s,
including actual 65,535-glyph Source Han grid coverage (1.7 s).

Preliminary desktop evidence at `84e881e`: embedded debug Tauri build passed
in 63.52 seconds; the native shell launched with isolated configuration and
loaded DejaVu Sans through its real Open chooser, displaying 6,253 glyphs. The
native Save chooser opened, but export was not completed. The user reported
that automation competed with their own input, so all native/headed input was
stopped and the owned process/dialogs/config/output were cleaned up. Remaining
native interaction is **paused and unverified**, not passed. Continue only with
isolated headless browser, unit and CLI checks; no further desktop interaction
is authorized in this run. Final embedded debug Tauri building at `2e53ba6`
passed in 16.03 s without launching a window. The artifact is preserved at
`target/p1-acceptance/fontcutter-desktop`; fresh release CLI/server binaries are
in `target/release`. Release packaging has not been tested.

## Final verification and handoff

Integrated Rust verification on 2026-09-07 passed **453 tests**, with two
explicitly ignored release-only acceptance tests covered by their separate
release executions above. `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets -- -D warnings` also passed, including
a fresh formatting/Clippy run after the final TTC correction. The nine final
preservation tests were independently rerun; the full 453-test count describes
the earlier integrated run. An earlier
attempt failed during linking when the project directory briefly became
inaccessible; the full retry after path recovery is the passing result recorded
here, rather than the interrupted run.

Detailed commands and outputs are recorded in the tracked P1 ledger snapshots:
`task-12-kernel-report.md`, `task-12-kernel-review.md`,
`task-9b-shaping-identity-report.md`, `task-9b-kernel-review.md` and
`fixture-path-resume-report.md` and `task-12-cjk-report.md`.

Final commands and results: `pnpm check`, `pnpm lint`, `prettier --check .`,
`pnpm test` (930 passed), and `FC_API_PORT=8787 FC_UI_PORT=1428 pnpm exec
playwright test --workers=1 --reporter=list` (21 passed, no skips). The release
CLI/server and embedded desktop artifacts contain the final corrected Rust/UI.

The final scoped reviewer independently passed TypeScript, 57 app tests and
nine preservation tests. Earlier concurrent builds caused short-wait view-test
failures; unchanged tests passed once heavy builds drained. Failed diagnostic
runs were retained and are not counted as acceptance passes.

This concludes the authorized P1 run. P2, main merge and historical fixture
history changes remain outside this handoff. Native interaction stays paused.

## Final review corrections

Whole-branch review identified three blocking persistence/handover defects.
The fix wave serializes save/export/close with old tools and document edit
barriers, checks fresh kernel dirty state before closing, declines ambiguous
TTC source-face preservation, and finishes an old-layer Pen path before an
inner-toolbar master switch. Real controller and generated TTC regressions
cover the original triggers; scoped re-review approved all three corrections
at `2e53ba6432b9bee3aa15000a7918f472c9289fa9`.

The drag test now waits for the successful asynchronous `doc.edit_end` response
after collecting timed pointer samples, then checks exact persisted coordinates
and undo. A 40 ms-per-apply diagnostic demonstrated 220 successful sequential
requests taking 14.941 s to close: the old five-second polling window could
expire before completion. This wait is outside the unchanged 8 ms input/paint
budget. The old failure and its cleanup-generated NotFound messages are retained
as diagnostic evidence; neither coordinate assertions nor timing limits were
relaxed.

The preview uses model outlines with kernel-shaped advances at 12/24/48 px;
it is not a hinted compiled-font rasterizer. Visible path work is capped at
512 glyphs and the preview backing window at 4096 px. Kerning editing, vertical
text and the recorded P2 backlog are deferred.
