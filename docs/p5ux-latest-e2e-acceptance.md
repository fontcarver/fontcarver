# Latest-source full browser regression

2026-10-04. Scope: the user's selected full E2E follow-up after the workspace
200% fixes, including corrections to two gesture-test setup steps. The default configuration discovers **125 tests in 30 files**,
including the P2 kerning/source/format specs and all 20 zoom regressions added
since the previous 105-test milestone.

## Source and execution

The tested tree is `56442b5` plus the local workspace-zoom changes and E2E gesture-test corrections, not the bare
commit. The [source manifest](acceptance/p5ux/latest-e2e-20261004/source-manifest.json)
records 2,065 non-documentation repository inputs (including untracked source),
their SHA-256 values and both executable hashes. Its aggregate source digest is
`86e67ddfcc6239a40456fab73b88bc43ca3ba434b2d3009ebf4b5da31eeee951`.
All recorded source inputs and both executables still match after the final run.

Vite serves current frontend source. The existing release `fc-devserver` and
CLI are reused: Rust/Cargo inputs have no differences from the previously
accepted `f6f6cf9` source. Each case owns its kernel/configuration/font sidecars;
one worker runs all cases with retries disabled. The external Source Han Sans
SC font is available and explicitly selected. No test thresholds are changed.

```sh
mkdir -p .scratch/latest-full-e2e/tmp
TMPDIR="$PWD/.scratch/latest-full-e2e/tmp" \
FC_API_PORT=9878 FC_UI_PORT=1528 \
FC_E2E_CLI="$PWD/target/release/fontcarver" \
FC_SOURCE_HAN_SANS="$PWD/../SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf" \
FC_PERF_TIMINGS=1 \
PLAYWRIGHT_JSON_OUTPUT_NAME="$PWD/.scratch/latest-full-e2e/final-results.json" \
pnpm --dir app exec playwright test --retries=0 --reporter=list,json \
  --output="$PWD/.scratch/latest-full-e2e/final-results"
```

**125 passed (9.0m), zero failures, skips or retries**
([complete output](acceptance/p5ux/latest-e2e-20261004/final-e2e.log),
[machine-readable results](acceptance/p5ux/latest-e2e-20261004/final-results.json)).
The real external CJK case ran. Vite's connection-reset messages accompany
per-test kernel teardown; they did not fail the run.

## Earlier complete runs and test corrections

The [first complete run](acceptance/p5ux/latest-e2e-20261004/initial-e2e.log)
finished **124 passed / 1 failed / 0 skipped**, with retries disabled. Performance
and the real CJK case passed. The failure was in the kerning workflow's initial
canvas click: the gap picker remained unselected
([failure](acceptance/p5ux/latest-e2e-20261004/kerning-failure.md),
[page before cleanup](acceptance/p5ux/latest-e2e-20261004/kerning-before.png)).

Shaping reveals the kerning controls, reducing canvas height after its initial
fit. The helper assumed its calculated handle was still clickable. It now waits
for the gap picker to be enabled, invokes the existing **Fit Glyph in Window**
control (also used by the beam-drag test), and checks `elementFromPoint` returns
the canvas before clicking. It still uses a real pointer click and checks the
selected gap; numeric edits, coalesced undo, saving, export and reopening keep
their original assertions. No synthetic selection or test timeout relaxation
replaces those checks.

The corrected kerning case passed **3 consecutive runs, zero retries**
([focused output](acceptance/p5ux/latest-e2e-20261004/focused.log),
[reachable selected handle](acceptance/p5ux/latest-e2e-20261004/kerning-after.png)).
It also passed the [second complete run](acceptance/p5ux/latest-e2e-20261004/second-e2e.log),
which finished **124 passed / 1 failed / 0 skipped**. That run exposed a different
intermittent failure in Open Corner's marquee setup
([failure](acceptance/p5ux/latest-e2e-20261004/corner-failure.md)).

The corner test had waited for the kernel's point count, then immediately dragged
around an earlier screen position. It now waits for the visible selected-node
coordinates to match the updated outline, respecting their one-decimal display,
and computes the marquee from the two actual mouth nodes and the current canvas
transform. Both endpoints must hit the canvas. Real pointer selection of two
nodes, menu-driven reconnection and both undo assertions remain. The first
probe incorrectly read transient drag-preview data (null between gestures);
that probe was replaced with the visible-coordinate wait. The corrected corner
case passes **3 consecutive runs, zero retries**
([output](acceptance/p5ux/latest-e2e-20261004/draw-focused.log)).

The final complete run follows both corrections. Relative to the initial run,
only `app/e2e/p2aSupport.ts` and `app/e2e/draw.spec.ts` changed. Both earlier
source manifests are retained
([initial](acceptance/p5ux/latest-e2e-20261004/initial-source-manifest.json),
[second](acceptance/p5ux/latest-e2e-20261004/second-source-manifest.json)).

## Performance coverage

The [browser drag gate](../app/e2e/acceptance-performance.spec.ts) uses a
300-point glyph, 20 warm-up moves and 200 measured moves paced by animation
frames. It checks synchronous repaint and changing preview coordinates on every
move, committed final coordinates and undo. The original pointer-down, move P95
and move maximum limits remain **8 ms**.

| Measurement            | This complete run |
| ---------------------- | ----------------: |
| Move median            |            1.1 ms |
| Move P95               |            1.4 ms |
| Move maximum           |            2.6 ms |
| Moves over 8 ms        |           0 / 200 |
| Pointer-down           |            4.5 ms |
| Renderer P95 / maximum |      0.8 / 2.2 ms |

[All 220 diagnostic samples](acceptance/p5ux/latest-e2e-20261004/performance.json)
include warm-up; the move statistics above use the 200 measured samples.
The timer measures synchronous browser event handling and canvas drawing.
It excludes frame scheduling delays, asynchronous kernel request completion and
final screen presentation; passing does not establish an end-to-end input
latency or FPS guarantee.

The [CJK browser case](../app/e2e/cjk.spec.ts) opens **65,535 glyphs** and reaches
the last grid cell. This is a large-font functional check with timeouts (20 s
for the glyph count and 10 s for the final cell), not a sampled throughput or
memory benchmark.

Additional performance coverage already exists outside E2E; it was inspected,
not rerun in this task:

- [Kernel edits](../crates/fc-api/tests/edit_performance.rs): 20,000 glyphs,
  300 moved points, 200 samples, release-only P95 at most 4 ms.
- [Incremental layout](../crates/fc-layout/tests/incremental_layout_budget.rs):
  refresh below 750 ms and cheaper than full compilation; 20 consecutive edits
  below 6 s.
- [Large-font import](../crates/fc-sfnt/tests/read_cjk.rs) and
  [export](../crates/fc-compile/tests/roundtrip_shaping.rs): release limits of
  3 s and 60 s respectively, plus separate Criterion
  [open](../crates/fc-sfnt/benches/open.rs)/
  [export](../crates/fc-compile/benches/export.rs) benchmarks.

Long-duration memory growth, representative editing of large fonts, physical
trackpads/high-DPI displays and native platforms remain separate backlog work.
The prior 2,917-test frontend/build checkpoint is unchanged; no application code,
translations or dependencies changed in this task. The E2E helpers now have
stronger readiness and hit-target assertions; existing functional/performance assertions remain.
Type checking and the changed tests' ESLint/Prettier checks pass
([output](acceptance/p5ux/latest-e2e-20261004/static-checks.log)).
Linux installers were not rebuilt. This task does not alter the 112-row audit
or the broad project-effort estimate.

## Resources and cleanup

Initial gate: **5.33 GiB build / 12,262 files, 0.00 GiB temporary / 2 files,
532.10 GiB free, PASS**. Owned test/kernel/browser processes have stopped and ports 9878/1528 are free.
After retaining the evidence above, this task's scratch directory was removed
(12,952,608 bytes / 949 files). Documentation formatting,
139 local links/anchors and whitespace checks pass. Final resource gate: **5.33 GiB build / 12,262 files, 0.00 GiB temporary /
2 files, 529.27 GiB free, PASS**.
