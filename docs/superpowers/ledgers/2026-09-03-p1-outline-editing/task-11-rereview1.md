# Task 11 — fix round 1 re-review

Diff reviewed: `067af1b..e8ad7c5` (6 commits, 14 files, +226/-28), via
`review-067af1b..e8ad7c5.diff` (724 lines, read whole, no hunk cut-off).

### Finding Verdicts

- **Important 1 — `invalidateFontInfo` could re-cache a stale response.** ADDRESSED —
  `app/src/stores/fontInfoCache.ts:19-24` adds a per-document `revisions` map;
  `:26-28` `revisionOf`; `loadFontInfo` (`:36-62`) stamps `issued = revisionOf(doc)` at
  `:42` and only writes the cache / clears `inFlight` when `current()` still holds
  (`:50-53` on success, `:56-59` on rejection); `forgetFontInfo` (`:68-72`) bumps the
  revision and deletes `inFlight` unconditionally; `invalidateFontInfo` (`:91-94`) calls
  `forgetFontInfo` then notifies listeners. The implementer correctly went beyond the
  review's suggested one-liner (`inFlight.delete(doc)` alone would have let the stale
  `.then` still `cache.set` and its `.finally` evict the newer request) — verified by
  the new `app/src/stores/fontInfoCache.test.ts` (4 cases): shared in-flight request
  (`:29-40`), invalidation forces a second request (`:42-51`), a response landing after
  its invalidation is never cached and the fresh one is (`:53-71`), and the newer
  request is not evicted when the older one later settles (`:73-91`).

- **Important 2 — CI's "upload report" step always uploaded nothing.** ADDRESSED —
  `app/playwright.config.ts` adds `reporter: process.env.CI ? [['html', { open: 'never'
  }], ['list']] : 'list'`, `use: { trace: 'on-first-retry', screenshot:
  'only-on-failure' }`, and `retries: process.env.CI ? 1 : 0` (necessary and disclosed:
  without a retry, `on-first-retry` never fires). Upload paths in
  `.github/workflows/ci.yml` (`app/playwright-report`, `app/test-results`) are
  untouched by this diff and match the html reporter's defaults. Report claims the
  artifact was verified empirically (`CI=1 … pnpm --dir app e2e` → `playwright-report/
  index.html`, 520 KB) — consistent with the config.

- **Minor 2 — no `timeout-minutes` / `forbidOnly` / `globalTimeout`.** ADDRESSED —
  `.github/workflows/ci.yml:15` `timeout-minutes: 45` on the `verify` job, `:111`
  `timeout-minutes: 20` on the "Run end-to-end tests" step; `app/playwright.config.ts`
  adds `forbidOnly: !!process.env.CI` and `globalTimeout: 15 * 60 * 1000`. See New
  Breakage for a scoping nuance on the latter two.

- **Minor 3 — `cjk.spec.ts` comment/budgets said "debug binary".** ADDRESSED —
  `app/e2e/cjk.spec.ts` comment now reads "The skip above guarantees a release kernel…"
  and budgets dropped from 240 000/180 000/60 000 ms to 60 000/20 000/10 000 ms,
  consistent with the release-kernel skip already in place.

- **Minor 4 — transport failures reused the `io` code.** ADDRESSED —
  `app/src/transport/HttpTransport.ts:55-59` throws `ApiCallError('timeout', …)` on
  abort, `:61-64` throws `ApiCallError('transport', …)` on any other fetch failure.
  New nested (not flat-dotted) keys `errors.timeout` / `errors.transport` in both
  `app/src/i18n/en.json:250-251` and `app/src/i18n/zh-CN.json:250-251` (zh-CN carries
  English placeholders, untranslated, as ruled). Grepped the whole `app/src` tree for
  `'io'` and `code ===` matches: the only remaining `'io'` usages are
  `Toast.test.tsx:76` and `errors.test.ts:48`, both exercising the kernel's genuine
  file-I/O code (`crates/fw-api/src/error.rs:27`), not the transport condition this
  finding was about — no consumer matched on the old code for a network condition.
  Pre-existing `status.transport` key (`en.json:206`, a different namespace: "Kernel:
  {{kind}}") does not collide with the new `errors.transport` (nested under a different
  parent object).

- **Minor 8 — toast stack uncapped and 420 px.** ADDRESSED —
  `app/src/components/ToastStack.tsx:9` `MAX_VISIBLE = 3`, `:18` `toasts.slice(-MAX_VISIBLE)`.
  Confirmed this drops the *oldest*, not the newest: `pushToast` appends
  (`app/src/stores/uiStore.ts:319` `[...state.toasts, {...}]`), so `slice(-3)` keeps the
  three most recently pushed. `app/src/components/controls.module.css:191` narrows
  `width: min(420px, …)` to `min(320px, …)`. Styling stays token-based throughout
  (`var(--space-*)`, `var(--color-*)`); only the literal pixel cap changed, same pattern
  as before. Test `Toast.test.tsx:47-59`/(diff `:202-214`) pushes five, asserts three
  render and that `'five'` is present while `'two'` is gone.

- **Minor 1 — `invalidation.ts` docstring overstated.** ADDRESSED —
  `app/src/views/font/invalidation.ts` (diff `:699-715`) now reads "This is the single
  place the font view and the Font Info tab read a `Change`" and explicitly names the
  other readers (`sessionStore`, `useGlyphDetail`, `useLayerOutline`, `glyphPager`) —
  exactly the four sites the original review cited as omitted.

### New Breakage in the Fix Diff

None Critical or Important. Two trivial items:

- **Trivial** — `app/playwright.config.ts`: `globalTimeout: 15 * 60 * 1000` and
  `use.screenshot: 'only-on-failure'` are unconditional, not gated on
  `process.env.CI`, so both now apply to local runs too (previously: no global
  timeout, no screenshot capture locally). This is a small deviation from the review
  scope's "behaves the same locally as before except for CI" — in practice harmless:
  15 minutes is generous for the local suite (~45 s) and a screenshot only writes on a
  failing local run, which is a debugging aid, not a behavior change to passing runs.
  `retries` and `trace` remain effectively no-ops locally (`retries: 0` means
  `on-first-retry` never fires), so those two are unaffected.
- **Trivial** — `app/src/stores/fontInfoCache.ts:24` `revisions` is a plain `Map` that
  is only ever bumped (`forgetFontInfo:69`), never pruned, unlike its sibling maps
  `cache`/`inFlight` which call `.delete(doc)` on the same path. Every document ever
  closed leaves one numeric entry behind for the life of the tab. Negligible in
  practice (one number per document) and not a correctness issue for the fix itself,
  but inconsistent with the bounded-LRU precedent (`scrollMemory.ts`, 8-entry cap) the
  original review praised elsewhere in this task.

### Out-of-Scope Observations

- `app/src/editor/editSession.ts:113` — the pre-existing unhandled-rejection bug
  (Task 6's territory, already flagged in the base review as pre-existing and outside
  this task) is now reported by the implementer to reproduce far more often after this
  fix round (3/3 local runs vs. ~1/3 before), and with a more legible message, because
  (a) the new `fontInfoCache.test.ts` file shifts Vitest's worker scheduling and (b)
  Minor 4's wrapping turns the escaping rejection into a named `ApiCallError: …
  transport` instead of an opaque `TypeError`. This is a visibility/frequency change
  caused by this fix round's otherwise-correct work, not a new bug introduced by it —
  root cause and fix ownership remain Task 6's, and the implementer disclosed it
  transparently in "Concerns (fix round 1)".
- `app/e2e/cjk.spec.ts` hard-coded absolute fixture path — pre-existing, already
  disclosed by the implementer, unchanged by this round.

### Verdict

**Fix round:** All findings addressed, no new Critical/Important breakage. Two trivial
observations noted above (Playwright config's `globalTimeout`/`screenshot` not gated
on CI; the ungrowing `revisions` map) — neither blocks acceptance.
