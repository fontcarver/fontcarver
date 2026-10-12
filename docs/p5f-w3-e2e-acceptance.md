# Wave 3 browser and external CJK verification

Date: 2026-10-02. Task: `p5f-w3-e2e`.

The three previously failing download workflows pass in the current full-access
execution environment with the unchanged bundled Chromium headless shell
**151.0.7922.34** and Playwright **1.62.1**. A standalone blank-page Blob download
also succeeds and its saved bytes equal `fontcarver download probe`. No product
download workaround, retry, skipped assertion or browser dependency change was
used. The earlier cancellation remains an observed failure of that earlier
environment; these results do not establish its precise cause.

Full Chromium and installed Google Chrome were also probed headlessly. They
cannot launch under this checkout's long task-local TMPDIR because their Unix
singleton socket pathname exceeds the platform limit. The bundled headless
shell works with both the normal and a deliberately longer task-local TMPDIR;
there is no reason to switch the suite to a different browser.

The external Source Han Sans SC Regular font already exists outside the
repository. Its own name table identifies version **2.005** and **SIL OFL 1.1**;
FontTools reports **65,535 glyphs**. SHA-256:
`f1d8611151880c6c336aabeac4640ef434fa13cbfbf1ffe82d0a71b2a5637256`.
A fresh release kernel enables the existing test, which imports this font and
renders its final grid cell. The font is neither copied into Git nor modified.

All browser commands below use `FC_API_PORT=8851`, `FC_UI_PORT=1461`, absolute
`TMPDIR=$REPO/.scratch/p5f-w3-e2e/tmp`, and an explicit output directory under
that task scratch directory.

The final complete command, run from the repository root, was:

```sh
FC_API_PORT=8851 FC_UI_PORT=1461 \
FC_E2E_CLI="$PWD/target/debug/fontcarver" \
FC_SOURCE_HAN_SANS=/home/chen/githubprojects/fontpixel/SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf \
TMPDIR="$PWD/.scratch/p5f-w3-e2e/tmp" \
pnpm --dir app exec playwright test --retries=0 \
  --output="$PWD/.scratch/p5f-w3-e2e/final-results"
```

| Verification | Result |
| --- | --- |
| `pnpm --dir app exec playwright test e2e/bitmap.spec.ts e2e/text.spec.ts --grep 'sprite sheet round trips\|self-contained\|vector PDF' --repeat-each=3` | **9 passed**, 2.8 minutes; actual PNG import round trip, embedded HTML font loading, vector PDF and vertical outline HTML assertions preserved. |
| `flock .scratch/p5f/cargo.lock cargo build --release -p fc-devserver` | Passed, 3 minutes 18 seconds. |
| Same build after the backend fix freeze | Passed, 59.59 seconds; the integration owner subsequently rebuilt the final release kernel and CLI before both complete browser runs. |
| `pnpm --dir app exec playwright test e2e/cjk.spec.ts --grep 'opens Source Han Sans'` | **1 passed**, 16.4 seconds total; no external-font skip. |
| `pnpm --dir app exec playwright test e2e/spacing-shortcuts.spec.ts --repeat-each=3` | **3 passed**, 57.8 seconds with the rebuilt kernel; real-kernel metrics, undo/redo, lock and navigation verified. |
| Bitmap follow-up cases | **2 passed**: actual Save PNG… download/import round trip and vertical canvas pixel positions. The accompanying spacing case exposed the readiness issue described below. |
| First complete default suite | **99 passed, 1 failed, 0 skipped**, 5.1 minutes. Only the strict pointer-to-paint maximum failed; details below. |
| Focused performance repeat, `FC_PERF_TIMINGS=1`, `--repeat-each=3 --retries=0` | **3 passed**, 54.2 seconds; all 600 measured samples remain within the original 8 ms limit. |
| Second complete default suite | **100 passed, 0 failed, 0 skipped**, 24 files, **5.2 minutes**, exit 0; zero retries and diagnostics disabled. |

The new spacing regression uses a 2.5-unit base preference and both selected
sidebearings. It covers ×100, exactly one unit, separate settled undo/redo,
Ctrl+] navigation and the spacing lock. Its initial fixture stayed in proofing
mode rather than selecting an editable glyph, so that failed run is recorded
as a harness setup error. The corrected setup selects the first shaped glyph;
the spacing task's failing unit tests establish its old-mapper baseline.
Document navigation follows font order rather than the proof's `AV` string; the
final test verifies that navigation changes the active glyph and leaves every
fixture glyph's metrics intact. Each key gesture waits for the visible Inspector
LSB and shaping reload, as a closed kernel transaction alone does not mean that
the next glyph editor is ready. The three final repetitions include that fix.

The bitmap follow-up updates the PNG action's expected role to **Save PNG…**,
retaining the original file-content/import assertions. Its new vertical test
seeds two real bitmap glyphs with distinct vertical origins and verifies all 24
dark canvas pixels: two upright 3×4 rectangles at `(2,15)` and `(0,29)` on a
10×44 canvas. This exercises the real kernel, layout and canvas renderer.

## Performance failure and diagnostic repeat

The first complete run failed on one of 200 pointer samples: median **0.9 ms**,
p95 **1.2 ms**, maximum **10.8 ms**, versus the unchanged **8 ms** maximum.
Pointer-down was **3.1 ms**; the separately timed renderer maximum was
**1.9 ms**. The other 99 tests passed, including the real external CJK case and
all downloads. This failed run is not counted as a passing gate.

The total timer covers event dispatch, React's pointer handler, coordinate and
drag-geometry work, drawing and subsequent synchronous work. The existing
renderer timer covers the draw body; it excludes preparation before that timer
and paint-statistics aggregation afterward. Its summary retains a rolling 240
paint samples; the first run did not retain per-pointer correlation. A difference
between these summaries therefore neither locates the original spike nor
identifies garbage collection or operating-system scheduling as the cause.

An optional `FC_PERF_TIMINGS=1` diagnostic now logs every sample's position,
total time, synchronous renderer time, paint count and preceding frame interval.
It leaves the warm-up count, 200 measured samples, persistence/undo assertions
and all 8 ms limits unchanged. Three focused zero-retry repetitions produced:

| Repeat | Pointer maximum | Gesture sample, including 20 warm-ups | Same-sample renderer |
| --- | --- | --- | --- |
| 1 | 1.9 ms | 33 and 92 | 0.6 and 1.6 ms |
| 2 | 2.1 ms | 193 | 1.8 ms |
| 3 | 2.4 ms | 164 | 1.3 ms |

The 10.8 ms spike did not recur. Recorded frame intervals varied, including
roughly 14–78 ms in the slowest samples, but those intervals lie outside the
measured synchronous dispatch. The evidence supports an unreproduced transient
timing spike, not a demonstrated renderer regression or a proven GC/scheduler
cause. It is unrelated to evidence for kernel-session isolation. The second
complete run passed with the default, diagnostics-disabled path and zero retries:
median **0.9 ms**, p95 **1.2 ms**, maximum **6.2 ms**, no samples over 8 ms;
pointer-down **3.0 ms**, renderer p95 **0.7 ms**, renderer maximum **1.7 ms**.

Exact result excerpts retained from the three validation stages:

```text
First complete run:
pointer-to-paint 300 points/200 frame-paced samples: median=0.900ms p95=1.200ms max=10.800ms over8=1 pointerDown=3.100ms rendererP95=0.700ms rendererMax=1.900ms
Error: expect(received).toBeLessThanOrEqual(expected)
Expected: <= 8
Received:    10.799999952316284
1 failed
99 passed (5.1m)

Focused diagnostic repetitions:
3 passed (54.2s)

Second complete run:
pointer-to-paint 300 points/200 frame-paced samples: median=0.900ms p95=1.200ms max=6.200ms over8=0 pointerDown=3.000ms rendererP95=0.700ms rendererMax=1.700ms
100 passed (5.2m)
```

Focused ESLint, Prettier and `tsc -p tsconfig.node.json` pass. Interim full
`pnpm check` runs encountered other tasks' unfinished bitmap module and API
generation; these are not passing integrated typechecks.

Resource checkpoint after the release kernel build: **15.59 GiB build**,
**0.31 GiB combined task scratch**, **606.09 GiB free**, checker **PASS**.
After the final complete browser run: **16.56 GiB build**, **0.05 GiB combined
task scratch**, **605.22 GiB free**, checker **PASS**. The reusable build cache
remains above the 10 GiB warning and below the 20 GiB ceiling; no further build
is part of this task.

Cleanup removed this task's **7.83 MiB / 610 files** of superseded logs, generated
fonts, downloads and browser reports after retaining the acceptance evidence
above. Ports **8851 / 1461** have no listeners. The final checker reports
**16.56 GiB build**, **0.04 GiB combined task scratch**, **605.23 GiB free**,
**PASS**. Shared build/dependency caches and the external source font remain
untouched. Work stops here as requested; package installation, publication and
native macOS/Windows execution were not undertaken by this browser task.

After the parent's integrated Rust build exposed duplicate Cargo feature caches,
browser CLI helpers were consolidated around prebuilt executables. Set
`FC_E2E_CLI` to pin the desired binary; otherwise debug then release binaries are
checked. Missing binaries produce a prerequisite error, never an implicit
`cargo run`. The hint fixture also reads its generated file with Node rather
than invoking Unix `cat`. The documented initial build now includes `fc-cli`.
