# Task 11 review — P0 UI backlog + self-contained e2e in CI

Diff reviewed: 2433b2b..067af1b (6 commits, 33 files, +1430/-403).

### Spec Compliance

❌ Issues found — one ruling-6 requirement is met only nominally (the Playwright report artifact is
always empty); everything else in the brief is present and verified.

Item by item (brief line 3 + dispatch rulings 1–7):

1. **`invalidateFor` extended to `change.masters`** — ✅ `app/src/views/font/invalidation.ts:48`
   (`if (change.font_info || change.masters) invalidateFontInfo(doc)`). Em-box refresh reaches the grid
   cells through `app/src/views/font/useFontMetrics.ts:62-66` (subscription) and `:81` (`revision` in the
   fetch effect's deps). Test: `app/src/views/font/GlyphGrid.test.tsx:348-377` asserts a second
   `doc.font_info` call after a `masters` event.
2. **`change.font_info` → Font Info live refresh** — ✅ `app/src/views/fontinfo/FontInfoPanel.tsx:53-58`
   hands the tab's own document events to the same `invalidateFor`; the reload comes back through
   `app/src/stores/fontInfoCache.ts:100-107`. Test: `app/src/views/fontinfo/FontInfoPanel.test.tsx:101-141`
   (Masters column header changes live). The pre-existing bug this fixes is real: the panel previously
   only watched `change.font_info`, so a master metrics edit left the Masters table stale.
3. **Single invalidation entry point** — ✅ for the caches this task owns. Only two call sites
   (`app/src/views/font/useGridPager.ts:90`, `app/src/views/fontinfo/FontInfoPanel.tsx:55`), the grid
   argument is optional, and the "no grid mounted at all" path is tested
   (`app/src/views/font/invalidation.test.ts:175-186`). ⚠️ the docstring's stronger claim is false — see
   Minor 1.
4. **`change.glyph_order` still handled** — ✅ the full-reload branch is untouched
   (`app/src/views/font/invalidation.ts:51-60` via `requiresFullReload`), covered by
   `invalidation.test.ts:76-88` and `GlyphGrid.test.tsx:379`.
5. **Scroll position survives a full reload** — ✅ `app/src/views/font/scrollMemory.ts` (51 lines) +
   `app/src/views/font/useScrollMemory.ts` (62 lines). Test `GlyphGrid.test.tsx:314-346` scrolls ten rows,
   emits a `glyph_order` change, simulates the browser's clamp to zero, and asserts `scrollTop === 720`.
   I verified the gate the design rests on: `pager.reset()` really does set the total back to `null`
   (`app/src/views/font/glyphPager.ts:150-155`), so `loaded` genuinely goes false during a reload and the
   clamped zero cannot overwrite the memory. Keyed per document + query (`glyphOrder.ts:29-36` includes
   `query.doc`), so a filter/sort/master change starts at the top and documents cannot collide. Deviation
   on storage location — see Deviations.
6. **Smooth-point display** — ✅ verified "already satisfied", claim accepted:
   `app/src/views/glyph/drawOutline.ts:302-310` draws `node.source.smooth` as a circle
   (`SMOOTH_RADIUS = 2.5` at `:50`) and a corner as a 5 px square (`CORNER_SIZE = 5` at `:49`), matching
   §4.1 item 9. `app/src/views/font/cellRenderer.ts` (185 lines) and `app/src/canvas/glyphPaint.ts` contain
   no node drawing at all — grep for `smooth`/`arc(` in both returns nothing — so the Font view cells were
   never a smooth-display site. `drawOutline` has exactly one caller
   (`app/src/views/glyph/renderGlyph.ts`), and `DEFAULT_OVERLAYS.points` is `true` (`renderGlyph.ts:37`),
   so the read-only glyph tab draws nodes too. Correct call to add no code.
7. **`GlyphGrid.tsx` split under 400** — ✅ `GlyphGrid.tsx` 155, `GridCells.tsx` 100,
   `useGridPager.ts` 96, `useGridPointer.ts` 211, `useGridKeys.ts` 92. The extractions are pure moves:
   selection (`useGridPointer.ts:129-160`), rubber band (`:162-206`), context menu (`:208-224`), keyboard
   (`useGridKeys.ts:64-89`) and cell layout (`GridCells.tsx:56-99`) are byte-for-byte the old bodies with
   the closed-over values turned into parameters. 15 pre-existing grid tests kept, 2 added (17 `it(` blocks).
8. **`ApiError.code` → i18n toasts** — ✅ `app/src/i18n/errors.ts:37-45` maps to `errors.<code>` with the
   raw message as `details`; unknown code → raw message, no disclosure (`errors.test.ts:24-31`).
   `app/src/stores/uiStore.ts:336-344` is the single conversion point, wired into the command reporter
   (`AppShell.tsx:46`), `sessionStore` (9 sites), `documentStore.ts:202` and `useGridPager.ts:61`; the old
   private `errorMessage`/`reportError` in `sessionStore` are gone with no dangling importers (grep:
   `errorMessage` has zero occurrences left). "Details" renders per §10 (`Toast.tsx:40-44`).
   **Key resolution verified**: `app/src/i18n/index.ts` sets no `keySeparator`, so i18next's default `.`
   applies and the nested `"errors": { … }` object at `en.json:240` / `zh-CN.json:240` resolves; grep finds
   no flat `errors.*` key anywhere in either catalogue, so the Task 6 shadowing trap is not repeated.
   zh-CN carries all 10 keys with English placeholders, untranslated as ruled.
   **Code coverage is exact**: `crates/fw-api/src/error.rs` produces exactly `not_found, invalid_params,
   io, parse, internal, geometry, export, shaping`; `HttpTransport.ts:90` adds `invalid_response`; both
   transports throw `ApiCallError` with a code (`TauriTransport.ts:34`), so the mapping works in the
   packaged app too.
9. **Self-contained e2e** — ✅ `app/playwright.config.ts:43-64`: `webServer` array, kernel first
   (`--port ${apiPort} --open fixtures/fonts/DejaVuSans.ttf`, `cwd` repo root, waits on `/health`, 180 s),
   Vite second with `FW_DEV_PROXY`/`VITE_FW_API`. Ports default to 8787/1420 (`:22-23`),
   `reuseExistingServer: !CI` on both. The same-origin plumbing is necessary, not decorative:
   `crates/fw-devserver/src/server.rs:43-47` allows only the `:1420` origins, and `app/vite.config.ts:24-29`
   proxies `/api` + `/events` to `FW_DEV_PROXY`. Binary preference release → debug → `cargo run`
   (`playwright.config.ts:33-40`) means a clean checkout needs only `cargo build -p fw-devserver`.
   `fixtures/fonts/DejaVuSans.ttf` exists (759 KB) after the fixture relicense.
10. **CJK spec keeps its skip** — ✅ plus a second skip (release kernel required). Deviation, justified;
    see Minor 3 for the stale comment and budgets left behind.
11. **CI runs `pnpm e2e` on Linux** — ✅ `.github/workflows/ci.yml:85-119`: `cargo build -p fw-devserver`,
    `playwright install --with-deps chromium`, `pnpm e2e`. Cargo registry/index/git cached on
    `Cargo.lock`+`rust-toolchain.toml` (`:31-41`), Playwright browsers cached on `pnpm-lock.yaml`
    (`:93-99`), pnpm already cached by `setup-node`. ❌ the artifact upload (`:114-124`) is a no-op — see
    Important 2. ⚠️ the ~15 min estimate cannot be verified from the diff.
12. **No kernel changes** — ✅ the diff touches nothing under `crates/**` or `src-tauri/**`; the additive
    flag allowance was not needed because `--port`/`--open` already exist
    (`crates/fw-devserver/src/main.rs:15-20`).

⚠️ Cannot verify from diff: CI wall time, the measured debug/release open timings, and the ~1-in-3 test
flake rate the report cites.

### Strengths

- The `invalidateFor` signature change is the right shape: `doc` instead of `query`, grid optional. It
  makes the "Font Info tab mounted, grid unmounted" case expressible rather than duplicating the rule, and
  that exact case has a test (`invalidation.test.ts:175-186`).
- The `masters` gap in the Font Info panel was a genuine pre-existing bug, found and fixed rather than
  worked around.
- The scroll-restore design attacks the real failure (the browser clamps to zero while the content has no
  height) instead of the symptom, and the test simulates the clamp rather than trusting jsdom to reproduce
  it (`GlyphGrid.test.tsx:337-341`).
- Ruling 1 was answered by investigation, not by writing code — and the investigation holds up under
  independent check (item 6 above).
- The split preserves every behaviour path and the extracted modules each carry a docstring explaining why
  they exist (`useGridPointer.ts:1-8` on why handlers live on the container, not the cells).
- The e2e config's IPv4-explicit URLs and CORS reasoning are correct for this kernel, not cargo-culted.
- The `--open` fixture path is genuinely immune to the 30 s transport timeout the report worries about:
  `crates/fw-devserver/src/main.rs:42-51` opens every `--open` font *before* binding the listener, so
  Playwright's `/health` wait covers the parse and no `HttpTransport` request is involved. The CJK spec is
  different because it opens through the UI dialog — the report's reasoning is sound.

### Issues

#### Critical (Must Fix)

None.

#### Important (Should Fix)

1. **`invalidateFontInfo` leaves the in-flight request, so stale data can be re-cached.**
   `app/src/stores/fontInfoCache.ts:64-67` deletes the cache entry and notifies listeners, but does not
   touch `inFlight` (`:17`, populated at `:38`). Sequence: edit A invalidates → a listener calls
   `loadFontInfo` → request R1 starts; edit B arrives 50 ms later → invalidate → cache deleted → listener
   calls `loadFontInfo` → `:29-30` returns the *pending* R1 → R1 resolves with pre-B data and writes it
   back into the cache at `:34`. The panel then shows pre-B data with nothing left to trigger another
   refetch. Same hazard on the first mount: a fetch in flight when the first edit event lands. Reachable
   with any rapid edit stream (held key, undo/redo, drag). The pattern predates this task (the old
   `useFontInfo` `reloadToken` path had it too), but this change is what makes invalidation event-driven
   and frequent, and "Font Info live refresh" is this task's own item. Fix is one line: `inFlight.delete(doc)`
   inside `invalidateFontInfo`.
2. **The CI "Upload the Playwright report" step can never upload a report** (ruling 6, plan-adjacent).
   `app/playwright.config.ts` sets no `reporter`, and Playwright 1.62.1's default is
   `reporter: [[process.env.CI ? 'dot' : 'list']]` (verified in
   `node_modules/.pnpm/playwright@1.62.1/…/lib/common/index.js:2513`) — the `html` reporter is never
   selected, so `app/playwright-report/` is never written. `use.trace`, `use.screenshot` and `use.video`
   are also unset, i.e. all off, so `app/test-results/` holds nearly nothing on failure either.
   `.github/workflows/ci.yml:120-124` has `if-no-files-found: ignore`, so the emptiness is silent and a red
   CI run gives a maintainer only the dot reporter's stdout. Fix: `reporter: process.env.CI ? [['html', {
   open: 'never' }], ['list']] : 'list'` plus `use: { trace: 'on-first-retry' }`.

#### Minor (Nice to Have)

1. **`invalidation.ts:28` overstates**: "This is the single place that reads a `Change`" is false —
   `app/src/views/glyph/useGlyphDetail.ts:113-118`, `app/src/views/glyph/useLayerOutline.ts:87`,
   `app/src/stores/sessionStore.ts:118` and `app/src/views/font/glyphPager.ts:279-280` all read `Change`
   fields directly. In particular `useGlyphDetail.ts:113` duplicates this task's own
   `font_info || masters` rule (which is what keeps the shared cache fresh while a glyph tab is the active
   one — so behaviour is correct, but the rule now lives in two places). Either fold that subscriber into
   `invalidateFor` or soften the docstring to "the single place the font view and Font Info tab read a
   `Change`".
2. **No time bound on the CI job or the suite**: `.github/workflows/ci.yml` has no `timeout-minutes`
   (GitHub's default is 360), and the Playwright config sets neither `globalTimeout` nor
   `forbidOnly: !!process.env.CI`. The webServer timeouts bound the common hang, but a stray `.only`
   would silently narrow the CI run to one test and pass.
3. **`e2e/cjk.spec.ts` contradicts itself.** The new skip at `:21-24` guarantees a release kernel, but the
   comment at `:29-31` says "The suite runs against the debug binary, so the budget is the debug one" and
   the budgets (`240_000` test, `180_000` open, `60_000` last page) are debug-sized. On the release kernel
   the spec is now allowed to take 4 minutes to fail on a real regression that would show in 3 s.
4. **`errors.io` will mis-describe transport failures.** `HttpTransport.ts:53-58` reuses the `io` code for
   a request timeout, so an unreachable kernel is summarised as "A file could not be read or written"
   (`en.json:243`) with the timeout text hidden under Details. Either widen the wording or give the
   transport its own code.
5. **Scroll offset resolution is one row, not one pixel.** `useScrollMemory`'s effect depends on
   `scrollTop`, which `useVirtualGrid.ts:208-227` only updates when the rendered row range changes, so the
   recorded offset is whatever the container held at the last row crossing. Restoring therefore lands up to
   one row off. Acceptable for the requirement; worth a comment.
6. **`reuseExistingServer` can reuse the wrong kernel locally**: a `fw-devserver` already running on
   `FW_API_PORT` *without* the fixture open satisfies the `/health` probe, and every spec then fails on a
   missing document. A one-line note in the config, or probing `session.list`, would save the confusion.
7. **`app/src/views/font/GlyphGrid.test.tsx` is 405 lines**, just over the 400-line guideline (well under
   the 800 cap). Pre-existing file that grew by 67 lines; the two new cases could move to a
   `GlyphGrid.invalidation.test.tsx`.
8. **Design §10 is still only partly met** (pre-existing, outside the brief's toast item, noted because
   §10 was the cited spec): `ToastStack.tsx:22-24` renders every toast with no "stack ≤ 3" cap, and
   `controls.module.css:186-193` is 420 px wide, not 320 px.

### Deviations judged

- **Scroll memory in a module-level LRU instead of `DocView`** (`scrollMemory.ts:19`, ruling 2's
  parenthetical): **accept**. It is a cache, not UI state, and it follows the file's own established
  precedent (`glyphOrder.ts`, `glyphNames.ts`), is bounded at 8 entries, and is released with the other
  per-document caches on close (`caches.ts:24`, `forgetScroll`, tested at `scrollMemory.test.ts:40-49`).
  The stated re-render argument checks out: `selectDocView` is subscribed by five views, so a store write
  per row crossed would re-render all of them. The "no new global state outside Zustand stores" constraint
  is aimed at UI state; this does not violate its intent. Recommend the controller bless it explicitly.
- **`useFontInfo` now keeps the previous answer during a refetch** (`fontInfoCache.ts:133-137`): an
  unrequested improvement, correctly guarded by `loaded.doc === doc` so one document's info cannot leak
  into another's panel.
- **`e2e/glyphview.spec.ts` rewritten onto Task 6's Info box** (`:26-29`): necessary — the `Glyph readout`
  HUD it asserted on no longer exists. `getByLabel('advance')` is a substring, case-insensitive match, so
  it survives an "Advance" label; it will not survive Task 6 renaming or restructuring the field, so flag
  the coupling to Task 6/7.
- **Second CJK skip**: acceptable and reported, but see Minor 3.

### Report claims re-checked

- "This will make the **new** CI job red at random" (report concern 1) is imprecise: `pnpm test` was
  already a CI step before this diff (`.github/workflows/ci.yml`, unchanged "Test frontend"), so the
  `editSession.ts:113` unhandled rejection is a pre-existing CI risk owned by Task 6, not something this
  task introduced. The e2e job does not run Vitest.
- "Fixed on the way: the Masters table previously reloaded only on `font_info`" — confirmed by the
  removed `event.change.font_info` guard in `FontInfoPanel.tsx`.
- "That is every code `crates/fw-api/src/error.rs` produces" — confirmed exactly (8 codes, no more).
- The e2e evidence in the report (9 tests, 8 passed, 1 skipped, 42.7 s) is present and complete; no gap.

### Assessment

**Task quality:** Needs fixes

**Reasoning:** Every brief item is implemented and independently verified, the grid split and the smooth-point
investigation are exemplary, but two small defects should land before this is trusted: the font-info cache
can re-cache stale data because invalidation ignores the in-flight request, and CI's failure artifact is
empty because no HTML reporter or trace is configured — the one thing ruling 6 asked for on a red run.
