# P1 acceptance

Status: **in progress**, 2026-09-07. This record covers the `p1-outline-editing`
branch. P1 is not complete until the pending integration, browser, localisation
and review items below are resolved. P2 is outside this session's scope.

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
isolated worktree. Preliminary pointer-driven browser counterparts passed; the final integrated
rerun remains pending T9.

## Preservation and shaping

Eight `fc-compile` preservation tests passed. Entirely untouched fonts with
verified original glyph set/order retain `glyf`, `loca`, `maxp`, global hint
programs and glyph instructions verbatim. **Any outline edit drops all
TrueType hinting and emits a warning**, including instructions in otherwise
untouched glyphs. This is the explicit Task 3 fix-1 C1 ruling, a limitation of
P1 relative to the original broader preservation wording. Edited-font tests
verify both global and per-glyph instruction removal.

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
| Pointer handler through synchronous repaint, 300 points | median 1.000 ms; p95 1.300 ms; max 1.700 ms | every sample ≤ 8 ms | Preliminary passed; rerun after T9 |

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
single undo. Pointer-down measured 3.500 ms, and both pointer-down and every
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
SFNT/UFO round trips. Preliminary browser acceptance passed three tests:
DejaVu pointer edits to A/B/C with export/reopen; new-font pen drawing and
600/620/640 widths with export, release CLI inspection and reopen; and the
300-point drag gate. T8 and independently checked Chinese translations are
merged; integrated app tests passed 92 files / 869 tests. Final text editing,
preview, translation top-up and desktop build/launch checks remain pending. Native dialog interaction will be reported separately from
automated browser or build evidence.

## Evidence and remaining completion steps

Integrated Rust verification on 2026-09-07 passed **453 tests**, with two
explicitly ignored release-only acceptance tests covered by their separate
release executions above. `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets -- -D warnings` also passed. An earlier
attempt failed during linking when the project directory briefly became
inaccessible; the full retry after path recovery is the passing result recorded
here, rather than the interrupted run.

Detailed commands and outputs are recorded in the tracked P1 ledger snapshots:
`task-12-kernel-report.md`, `task-12-kernel-review.md`,
`task-9b-shaping-identity-report.md`, `task-9b-kernel-review.md` and
`fixture-path-resume-report.md` and `task-12-cjk-report.md`.

- Finish T9b UI implementation and scoped review.
- Execute integrated browser/desktop acceptance.
- Finish T13 translation and a separate QA pass.
- Run whole-branch review, resolve its P1 findings and record final checks.
- Stop after P1 and hand off the branch; merging to `main` is a separate user decision.
