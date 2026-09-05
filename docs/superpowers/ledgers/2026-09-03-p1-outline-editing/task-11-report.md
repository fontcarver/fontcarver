# P1 Task 11 — P0 UI backlog + self-contained e2e in CI

Branch `p1-t11` in `/home/chen/githubprojects/pixelfonts/fontworkshop/.worktrees/p1-t11`, base `2433b2b`.
Six commits, nothing pushed. Ports used: kernel 8790, Vite 1423.

## Commits

| Commit | Subject |
| --- | --- |
| `726ee7d` | `feat(app): translate kernel error codes into toasts` |
| `e1abe8f` | `feat(app): refresh master metrics and Font Info from one invalidation point` |
| `9d6f6a2` | `feat(app): keep the glyph grid's scroll position across a full reload` |
| `7bf4bd8` | `refactor(app): split the glyph grid by responsibility` |
| `7f93212` | `test(app): make the Playwright suite start its own servers` |
| `067af1b` | `ci: run the Playwright suite on Linux` |

## Per brief item

### 1. `invalidateFor` extended to `change.masters` and `change.font_info` (ruling 2)

`app/src/views/font/invalidation.ts:41-71` — `invalidateFor(change, doc, generation, grid?)` is now the
single reader of `Change`. The pager argument became an optional `{ query, pager }` because only one tab's
content is mounted at a time (`app/src/shell/TabContent.tsx:13-17`): with the grid unmounted, the shared
caches still have to be dropped, so the Font Info tab hands its own document events to the same function
(`app/src/views/fontinfo/FontInfoPanel.tsx:52-60`) instead of rolling its own reload token. The grid's
call is `app/src/views/font/useGridPager.ts:85-92`.

`change.font_info || change.masters` invalidates the shared `doc.font_info` response
(`app/src/views/font/invalidation.ts:52`). The cache gained a subscription so both readers hear it:

- `app/src/stores/fontInfoCache.ts:56-79` — `onFontInfoInvalidated` / `invalidateFontInfo`.
- `app/src/stores/fontInfoCache.ts:100-160` — `useFontInfo` refetches on invalidation and keeps the
  previous answer on screen while it does, instead of flashing "Loading…".
- `app/src/views/font/useFontMetrics.ts:60-84` — the grid's em box reloads, so every cell repaints against
  the new ascender/descender.

Fixed on the way: the Masters table previously reloaded only on `font_info`, never on `masters`, so a
metrics edit left the Font Info tab stale.

Tests: `app/src/views/font/invalidation.test.ts:132-184` (four cases, including "no grid mounted at all"),
`app/src/views/fontinfo/FontInfoPanel.test.tsx:101-142` (Masters table live refresh),
`app/src/views/font/GlyphGrid.test.tsx:381-410` (a `masters` event refetches `doc.font_info`).

### 2. Scroll position survives a full reload (ruling 2)

`app/src/views/font/scrollMemory.ts` (51 lines) keeps one offset per filtered, sorted list, keyed by
`orderKey(query)`, LRU-bounded at 8 and released with the document's other caches
(`app/src/views/font/caches.ts:24`). `app/src/views/font/useScrollMemory.ts:34-62` records the offset while
the list is loaded and writes it back once the list knows its length again; while `pager.total` is `null`
it stops recording and re-arms the restore, so the zero the browser clamps to during the reload cannot
overwrite the memory. `useVirtualGrid` now returns `scrollTop` (`app/src/views/font/useVirtualGrid.ts:189`,
`:296`) so the hook re-runs once per row crossed rather than once per scrolled pixel. Because the key is the
list, a new filter, sort or master still starts at the top, and a tab switch now comes back where the user
left off.

**Deviation from the ruling's parenthetical.** The ruling suggested storing the offset "in the view state".
I kept it in a module-level per-query memory (the same shape as `glyphOrder.ts` / `glyphNames.ts`) rather
than in `DocView`: `selectDocView` is subscribed by `StatusBar`, `Sidebar`/`SidebarFilters`, `Inspector`,
`FontView` and the grid, so a store write per row crossed would re-render all of them while scrolling a
65k-glyph font. The observable behaviour the ruling asks for is unchanged.

Tests: `app/src/views/font/scrollMemory.test.ts` (5 cases) and
`app/src/views/font/GlyphGrid.test.tsx:344-379`, which scrolls ten rows down, emits a `glyph_order` change
and simulates the browser's clamp to zero before asserting the offset is restored.

### 3. Smooth-point display (ruling 1) — no code change needed

Everything that draws nodes already honours `Point.smooth`:

- `app/src/views/glyph/drawOutline.ts:302-305` draws a smooth node as a circle and
  `:306-309` a corner as a square; the sizes at `drawOutline.ts:50-51` (`CORNER_SIZE = 5`,
  `SMOOTH_RADIUS = 2.5`) are the design's §4.1 item 9 (5 px square / 5 px circle).
- `drawOutline` is the only node renderer in the app; its single caller is
  `app/src/views/glyph/renderGlyph.ts:98-107`, and every glyph tab — read-only included — starts with
  `DEFAULT_OVERLAYS` (`renderGlyph.ts:35-42`, `points: true`) via `app/src/stores/editorStore.ts:86`.
- `app/src/views/glyph/glyphScene.ts:169` passes the kernel's contours through untouched, so `smooth`
  reaches the renderer.
- The Font view's cell renderer draws **no** nodes at all: `app/src/views/font/cellRenderer.ts:110-176`
  paints the flattened `Path2D` plus baseline, label, stripe and badge, so `Point.smooth` has no bearing
  there. Same for `app/src/canvas/glyphPaint.ts`.

Already covered by `app/src/views/glyph/drawOutline.test.ts:227-243`
("draws corner nodes as squares and smooth nodes as arcs"). Per ruling 1, no code was added.

### 4. `GlyphGrid.tsx` split (ruling 3)

403 → 155 lines, split by responsibility, no behaviour change:

| File | Lines | Responsibility |
| --- | --- | --- |
| `app/src/views/font/GlyphGrid.tsx` | 155 | wiring, container element, focus bookkeeping |
| `app/src/views/font/useGridPager.ts` | 96 | page requests, match count, invalidation, dev probe |
| `app/src/views/font/useGridPointer.ts` | 211 | click/Shift/Ctrl selection, rubber band, context menu |
| `app/src/views/font/useGridKeys.ts` | 92 | arrows, paging, Home/End, Enter, Escape |
| `app/src/views/font/GridCells.tsx` | 100 | the visible cells and the band |

All 15 pre-existing `GlyphGrid.test.tsx` cases pass unchanged (two were added for items 1 and 2).

### 5. `ApiError.code` → i18n toasts (ruling 4)

`app/src/i18n/errors.ts:37-45` — `errorNotice` maps `ApiCallError.code` to `errors.<code>` and puts the
kernel's own wording under "Details"; an unknown code (a newer kernel, a transport-level failure) falls
back to the raw message with no disclosure. `app/src/stores/uiStore.ts:325-337` — `reportError` is the one
place a thrown value becomes a toast; `app/src/components/Toast.tsx:36-49` renders the disclosure
(design §10), styled at `app/src/components/controls.module.css:212-238`. Call sites converted:
`sessionStore.ts` (its private `reportError`/`errorMessage` are gone), `documentStore.ts:204`,
`AppShell.tsx:44`, `useGridPager.ts:61`.

New keys, added to `en.json` **and** `zh-CN.json` with the English text as placeholders (Task 13
translates; I translated nothing):

```
toast.details
errors.not_found      errors.invalid_params  errors.io
errors.parse          errors.internal        errors.geometry
errors.export         errors.shaping         errors.invalid_response
```

That is every code `crates/fw-api/src/error.rs` produces (`not_found`, `invalid_params`, `io`, `parse`,
`internal`, `geometry`, `export`, `shaping`) plus `invalid_response` from `HttpTransport`.

Tests: `app/src/i18n/errors.test.ts` (5 cases, one asserting a key exists for every code above) and
`app/src/components/Toast.test.tsx` (5 cases: the disclosure, its absence, and `reportError` end to end).

### 6. Self-contained e2e (ruling 5)

`app/playwright.config.ts` — `webServer` is now an array: `fw-devserver --port ${FW_API_PORT} --open
fixtures/fonts/DejaVuSans.ttf` (waiting on `/health`, 180 s budget, `cwd` the repo root) and Vite with
`FW_DEV_PROXY`/`VITE_FW_API` set so the UI is same-origin with the kernel on any port. Ports come from
`FW_API_PORT` (default 8787) and `FW_UI_PORT` (default 1420); `reuseExistingServer` is off under CI. The
binary is picked as release → debug → `cargo run`, so a clean checkout needs only
`cargo build -p fw-devserver` and `pnpm install`. Every URL names `127.0.0.1` explicitly, because both
servers bind the IPv4 loopback and `localhost` resolves to `::1` first on some runners.

Two spec fixes were unavoidable to make the suite green:

- `app/e2e/glyphview.spec.ts:26-29` asserted on `getByLabel('Glyph readout')`, a HUD the Task 6 editor work
  already removed (`glyphView.hud.label` is in the catalogue but rendered nowhere). It now reads the Info
  box that replaced it: `getByRole('region', { name: 'Info box' })`, `U+0041`, advance `1401`.
- `app/e2e/cjk.spec.ts:26-34` keeps its skip when the corpus is absent and gains a second one: it needs a
  **release** `fw-devserver`. Measured on this machine, a debug kernel opens
  `SourceHanSansSC-Regular.otf` (65,535 glyphs) in 18–75 s, while `HttpTransport`'s request timeout is
  30 s (`app/src/transport/HttpTransport.ts:20`), so against a debug kernel the spec fails on speed rather
  than behaviour. Release opens the same font in under 3 s (`docs/p0-acceptance.md`). Its budgets were
  raised at the same time (240 s test, 180 s open, 60 s last page).

### 7. CI (ruling 6)

`.github/workflows/ci.yml` — after the frontend build the Linux job runs `cargo build -p fw-devserver`,
restores/installs the Chromium browser (`pnpm exec playwright install --with-deps chromium`, cached at
`~/.cache/ms-playwright` on the `pnpm-lock.yaml` hash) and runs `pnpm e2e`; on failure
`app/playwright-report` and `app/test-results` are uploaded for 7 days. The Cargo registry, index and git
DB are cached on the `Cargo.lock` + `rust-toolchain.toml` hash, and pnpm is already cached by
`actions/setup-node`. The formatting step became `prettier --check .` to match the local verification
command, with `test-results`/`playwright-report` added to `app/.prettierignore`.

**`target` is deliberately not cached**: this workspace's `target` is 4.4 GB, which costs more to upload
and restore each run than it saves and would churn GitHub's 10 GB per-repository cache budget. If the job
ever gets tight, `Swatinem/rust-cache` (which prunes) is the next step.

**Release build not used (measured, per ruling 6).** `cargo build -p fw-devserver` in debug is effectively
free after `cargo test --workspace` has built the workspace, and the debug binary opens the CI fixture
(DejaVu Sans, 6,253 glyphs) in ~12 s cold, well inside the 180 s budget. A release build would add a
second full compilation of the fontc/fea-rs dependency tree — several minutes — to save a few seconds, so
debug it is. The one thing debug cannot do is the 65k-glyph CJK corpus, which CI skips anyway.

**Job time estimate:** ~15 min cold, ~12 min with a warm registry cache. The new steps add roughly
20 s (`cargo build -p fw-devserver` after `cargo test`), 30–90 s (browser install, cached vs not) and
~50 s (`pnpm e2e`, including ~13 s of kernel start-up) — about 2 minutes on top of the existing job.

## Verification

```
$ pnpm --dir app gen:api                                     # exit 0
$ pnpm --dir app check && pnpm --dir app lint \
    && pnpm --dir app test && pnpm --dir app exec prettier --check .
  Test Files  52 passed (52)
       Tests  408 passed (408)
    Duration  11.80s
  Checking formatting... All matched files use Prettier code style!   # exit 0

$ cargo build -p fw-devserver && FW_API_PORT=8790 FW_UI_PORT=1423 pnpm --dir app e2e
  Running 9 tests using 4 workers
  -  1 e2e/cjk.spec.ts:26:3 › CJK font view › opens Source Han Sans and renders cells at the end of the grid
  ✓  3 e2e/app.spec.ts:3:1 › shows the workspace shell (5.7s)
  ✓  2 e2e/fontview.spec.ts:9:3 › font view › renders the glyph grid for the pre-opened font (7.5s)
  ✓  4 e2e/glyphview.spec.ts:8:3 › glyph view › opens a glyph tab from the grid and draws its metrics (9.1s)
  ✓  5 e2e/fontview.spec.ts:25:3 › font view › searches by Unicode scalar (9.6s)
  ✓  6 e2e/glyphview.spec.ts:35:3 › glyph view › steps to the next glyph in the current order (12.2s)
  ✓  7 e2e/fontview.spec.ts:41:3 › font view › filters by a sidebar category and reports the count (7.5s)
  ✓  8 e2e/glyphview.spec.ts:54:3 › glyph view › shows the font info sections and searches across them (4.5s)
  ✓  9 e2e/fontview.spec.ts:64:3 › font view › reports the selection in the status bar (5.1s)
  1 skipped
  8 passed (42.7s)                                            # exit 0, 45 s wall including start-up
```

Unit suite: 408 tests, +22 from the base (5 error mapping, 5 toast, 5 scroll memory, 4 invalidation,
1 Font Info live refresh, 2 grid).

## Concerns

1. **Pre-existing flaky unhandled rejection, in Task 6's territory.** `pnpm test` intermittently exits 1
   (roughly one run in three on a loaded machine) with
   `TypeError: RequestInit: Expected signal … to be an instance of AbortSignal` and no assertion failure.
   I traced it by stubbing `fetch` in the test setup: the escaping call is
   `app/src/editor/editSession.ts:113` (`this.chain = this.chain.then(async () => { … await call('doc.edit_apply', …) })`),
   whose promise chain is started with `void this.flush()` at `editSession.ts:96` and has no `.catch`, so a
   rejected `doc.edit_apply` — here one that outlives its test's mock transport — becomes an unhandled
   rejection instead of a toast. `app/src/editor/**` is Task 6/7's, so I left it alone; **this will make the
   new CI job red at random** until it is fixed (one `.catch(reportError)` on the chain).
   `app/src/views/font/useGlyphSelection.ts:104,131` has the same shape (`void selectRange(…)`) and is in my
   area, but it did not fire in any run, so I did not widen the change.
2. The `e2e/cjk.spec.ts` corpus path is still hard-coded to `/home/chen/githubprojects/pixelfonts/…`
   (pre-existing). It skips cleanly elsewhere, but it is not portable.
3. The scroll memory deviates from the ruling's suggested storage location — see item 2 above for why.
4. `fixtures/fonts/DejaVuSans.ttf` is now load-bearing for CI, not just for local runs.

---

## Fix round 1

Review: `task-11-review.md` (Needs fixes — 2 Important, 8 Minor). Fixed: Important 1 and 2, Minor 1, 2, 3,
4 and 8. Minor 5, 6 and 7 were deferred by the controller and left alone; the scroll-memory deviation was
accepted. Six commits on top of `067af1b`, nothing pushed.

| Commit | Subject |
| --- | --- |
| `d6db905` | `fix(app): discard a font-info response its invalidation overtook` |
| `96bc4cb` | `fix(app): give transport failures their own error codes` |
| `fe77499` | `fix(app): cap the toast stack at three and narrow it to 320 px` |
| `6470e17` | `test(app): write a diagnosable Playwright report on CI` |
| `056bc26` | `ci: bound the verify job and the e2e step in time` |
| `e8ad7c5` | `docs(app): correct the invalidation entry-point docstring` |

### Important 1 — stale font info could be re-cached

The one-line fix the review proposed (`inFlight.delete(doc)`) is necessary but not sufficient: the
overtaken request's own `.then` still ran `cache.set(doc, staleInfo)` on arrival, and its `.finally`
would then have deleted the *newer* request from `inFlight`, so a third caller would have started a
duplicate. Fixed with a per-document revision instead.

- `app/src/stores/fontInfoCache.ts:20-29` — `revisions` map and `revisionOf`.
- `app/src/stores/fontInfoCache.ts:37-63` — each request stamps the revision it was issued for and writes
  the cache (and clears `inFlight`) only while that revision still stands; the rejection path is guarded
  the same way.
- `app/src/stores/fontInfoCache.ts:66-73` — `forgetFontInfo` bumps the revision and drops the in-flight
  entry, so `invalidateFontInfo`, the `onDocumentForgotten` teardown and the tests all get the same
  guarantee.

Tests: new `app/src/stores/fontInfoCache.test.ts` (4 cases) — request sharing still works; an invalidation
forces a second request rather than joining the pending one; a response that answers *after* its
invalidation is never cached and the fresh one is; and the older request settling does not evict the newer
one from `inFlight`.

### Important 2 — CI failure artifact was always empty

- `app/playwright.config.ts:47-58` — `reporter: process.env.CI ? [['html', { open: 'never' }], ['list']] :
  'list'`, `use: { trace: 'on-first-retry', screenshot: 'only-on-failure' }`. Added `retries: process.env.CI
  ? 1 : 0` as well, without which `on-first-retry` can never fire (Playwright defaults to zero retries, so
  no first retry exists).
- Upload paths confirmed against what Playwright actually writes: the html reporter's default
  `outputFolder` is `playwright-report` and the default `outputDir` is `test-results`, both relative to the
  config's directory, which is exactly `app/playwright-report` + `app/test-results` in
  `.github/workflows/ci.yml:114-124`.
- Verified empirically rather than by reading the config: `CI=1 … pnpm --dir app e2e` on a clean tree now
  produces `app/playwright-report/index.html`, 520 KB.

### Minor 2 — time bounds

- `.github/workflows/ci.yml:12-14` — `timeout-minutes: 45` on the `verify` job.
- `.github/workflows/ci.yml:110-112` — `timeout-minutes: 20` on the end-to-end step (≤ 30 as asked).
- `app/playwright.config.ts:49,52` — `forbidOnly: !!process.env.CI` and `globalTimeout: 15 * 60 * 1000`.

### Minor 3 — CJK budgets now match the release-kernel skip

`app/e2e/cjk.spec.ts:29-32,37,49` — the debug-sized budgets and the comment that contradicted the skip are
gone: `test.setTimeout(60_000)`, open `20_000`, last page `10_000`, sized for the release kernel the skip
above already guarantees (< 3 s per `docs/p0-acceptance.md`). A real regression now fails in seconds
instead of after four minutes.

### Minor 4 — transport failures no longer described as file I/O

`app/src/transport/HttpTransport.ts:52-66` — a request timeout raises `timeout` instead of reusing the
kernel's `io`, and a failed fetch (unreachable kernel) is wrapped as `transport` instead of reaching the
toast as a bare `TypeError` with no code at all. Additive on the TypeScript side only; no kernel change.
New keys `errors.timeout` and `errors.transport` in `en.json` and `zh-CN.json` (English placeholders,
untranslated, as ruled).

Tests: `app/src/transport/HttpTransport.test.ts:78-92` (timeout code) and `:54-66` — the pre-existing
"propagates network failures" case was rewritten rather than duplicated, since "propagate the raw error"
is precisely the behaviour this item replaces. `app/src/i18n/errors.test.ts:47-62` now asserts a key exists
for all 11 codes.

### Minor 8 — toast stack per design §10

`app/src/components/ToastStack.tsx:9-10,20` — at most the three most recent toasts render;
`app/src/components/controls.module.css:191` — 320 px instead of 420 px.
Test: `app/src/components/Toast.test.tsx:47-59` pushes five and asserts three render, newest kept.

### Minor 1 — docstring corrected

`app/src/views/font/invalidation.ts:26-33` — no longer claims to be the only place a `Change` is read; it
now says "the single place the font view and the Font Info tab read a `Change`" and names the other
readers (`sessionStore`, `useGlyphDetail`, `useLayerOutline`, `glyphPager`).

### Verification (fix round 1)

```
$ pnpm --dir app check && pnpm --dir app test && pnpm --dir app lint \
    && pnpm --dir app exec prettier --check .
  Test Files  53 passed (53)
       Tests  413 passed (413)   # +5 vs round 0: 4 font-info cache, 1 toast stack
      Errors  1 error            # pre-existing, see below
  Checking formatting... All matched files use Prettier code style!

$ FW_API_PORT=8790 FW_UI_PORT=1423 pnpm --dir app e2e
  1 skipped
  8 passed (17.7s)               # exit 0, 20 s wall including kernel start-up

$ CI=1 FW_API_PORT=8790 FW_UI_PORT=1423 pnpm --dir app e2e   # artifact check
  8 passed (1.0m); app/playwright-report/index.html written, 520 KB
```

### Concerns (fix round 1)

1. **The pre-existing `editSession.ts:113` unhandled rejection now reproduces on nearly every run**, so
   `pnpm test` exits 1 despite 413/413 passing. Measured: 1 of 3 runs at `067af1b`, 3 of 3 after this
   round. It is not a behaviour change — the escaping call is still
   `await call('doc.edit_apply', …)` inside the unguarded `this.chain` started by `void this.flush()`
   (`app/src/editor/editSession.ts:96,113`), reaching the real transport after its test's mock is torn
   down. Two things made it louder: the new test file shifts worker scheduling, and Minor 4's wrapping
   means the rejection is now a legible `ApiCallError: … transport` with a stack naming `editSession.ts:113`
   instead of an opaque `TypeError` from undici. `app/src/editor/**` is Task 6/7's, so I did not touch it.
   **The "Test frontend" CI step will be red until that chain gets a `.catch`** — one line, in their file.
2. `app/src/views/font/useGlyphSelection.ts:104,131` has the same `void`-an-async-call shape and is in my
   area; it never fired in any run, so I left it rather than widen a fix round.
3. `retries: 1` on CI means a genuinely flaky e2e test would pass on the retry. That is the price of
   `trace: 'on-first-retry'`; the trace and the HTML report still record the first failure, and the run is
   marked flaky rather than green-with-no-evidence.
