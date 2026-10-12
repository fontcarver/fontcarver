# T12 browser acceptance preparation — 2026-09-07

Preliminary evidence on `p1-acceptance-browser` through `ffaaa64`; final rerun after T9 integration remains required. The branch adds only `app/e2e/acceptance*` tests/helpers and no dependencies or production changes.

Fresh current-worktree release `fc-devserver` and CLI were built. With ports 8795/1429, isolated config and one worker:

`FC_API_PORT=8795 FC_UI_PORT=1429 pnpm exec playwright test e2e/acceptance-workflows.spec.ts e2e/acceptance-performance.spec.ts --workers=1 --reporter=list`

passed 3/3 in 15.5 s. The DejaVu workflow used the browser path prompt, pointer-edited real points in A/B/C, exported by dialog, inspected with the release CLI, reopened, and found every changed coordinate independent of TrueType contour rotation. The scratch workflow used New Font, Add Glyphs A-C, actual Pen closure, UI widths 600/620/640, dialog export, release CLI inspect/list, and reopened outline/width checks.

The 300-point gate used API setup, then actual DOM pointer events. Pointerdown had to begin a drag. Twenty warm and 200 measured moves were frame-paced; timing covered synchronous pointer dispatch, hit testing, preview creation and repaint, excluding animation-frame wait and Playwright/network latency. Every move changed preview and incremented required paint statistics synchronously; pointerup persisted and UI undo restored. Result: median 1.000 ms, p95 1.300 ms, max 1.700 ms, zero above 8 ms; pointerdown 3.500 ms; renderer p95 0.800 ms, max 1.400 ms. A tight burst diagnostic had four unexplained spikes above 8 ms and is not acceptance evidence.

`pnpm check`, ESLint, Prettier and `git diff --check` passed. Agent-browser visually confirmed DejaVu A, nodes, metrics, tools, layers and inspector. Native dialogs were not exercised.

Blocker: fresh release Source Han SC first `text.shape` took 76.779 s (HTTP 200, 4 glyphs, non-stale), exceeding `HttpTransport`’s fixed 30 s abort.
