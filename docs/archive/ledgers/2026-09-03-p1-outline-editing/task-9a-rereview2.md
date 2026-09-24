# Task 9a — Re-review 2 (fix round 2 verification)

Base `232a0c7` → head `8cb57dd`, one commit. Diff read from
`review-232a0c7..8cb57dd.diff`; current file contents read directly from the worktree
(`app/src/dialogs/NumberField.tsx`, `app/src/stores/prefsStore.ts`,
`app/src/components/Dialog.test.tsx`, `app/src/shell/dialog.module.css`,
`app/src/stores/uiStore.ts`). No git-history commands were run beyond the initial
`git status`/`git log` sanity check; no git state was mutated.

## Finding Verdicts

**Finding 1 — `NumberField` clamp wrong with min-only — ADDRESSED.**
`app/src/dialogs/NumberField.tsx:44-47` now reads exactly the ruled expression:
`Math.min(max ?? Number.POSITIVE_INFINITY, Math.max(min ?? Number.NEGATIVE_INFINITY, parsed))`
(`Number.POSITIVE_INFINITY`/`NEGATIVE_INFINITY` are `Infinity`/`-Infinity`, so this is the
ruling verbatim). `app/src/dialogs/NumberField.test.tsx` (new file) covers all four
permutations: min-only (`:26-29`, `-5→0`, `8→8`), max-only (`:31-34`, `50→10`, `-50→-50`),
both (`:36-40`, `50→10`, `-5→0`, `4→4`) and neither (`:42-45`, `-5→-5`, `9999→9999`), plus
the pre-existing empty-box guard. Ran this file directly: 5/5 pass.

**Finding 2 — `prefsStore.update` lost a patch when two updates raced the first read — ADDRESSED for the described scenario, but see New Breakage below.**
`app/src/stores/prefsStore.ts:60-77` adds a module-scoped `queue` (serial FIFO via
`queue = queue.then(task, task)`, both handlers so a rejection can't wedge it) and a
`pending` merge buffer. `update` (`:125-138`) merges `patch` into `pending` synchronously
at call time and enqueues a task that captures-and-clears `pending`, loads once if
unloaded, then does one `persist` with the merged record. `prefsStore.test.ts:111-141`
("coalesces updates that race one unresolved first read") reproduces exactly the ruled
scenario — two `update()` calls before a gated `prefs.get` resolves — and asserts one
`prefs.get`, one `prefs.set` carrying both patches, and final store state carrying both.
Ran this file directly: 10/10 pass, including this new test and the untouched
"reads the stored record before its first write" test for the single-update case.
Per-field "later wins" merge order (`pending = { ...pending, ...patch }`, later patch
spread last) is correct by inspection but not exercised by a test with two patches
touching the *same* field — minor gap, not a bug.

**Finding 3 — `Dialog.test.tsx` selected the chrome rule by position — ADDRESSED.**
`app/src/components/Dialog.test.tsx:31-48` now parses `dialog.module.css` into a flat
`RULES` array of `{selectors: string[], body}` (comma-split, whitespace-normalized) and
`declaration()` (`:50-56`) does `RULES.find(r => r.selectors.includes(selector))` — an
exact selector-string match, order-independent, instead of the old substring/position
regex scan. Verified against `app/src/shell/dialog.module.css`: the file legitimately has
both a standalone `.dialogHeader { padding: var(--space-3) ... }` (P0 chrome, line 21) and
a compound `.dialogSm .dialogHeader, .dialogMd .dialogHeader, .dialogLg .dialogHeader { padding: var(--space-5) ... }`
(line 126-130) — the old regexp's correctness for `declaration('.dialogHeader', ...)`
depended on the standalone rule appearing textually first in the file; the new exact-match
lookup does not. New guard test `:81-83` pins the flat-parse precondition (no at-rules in
the stylesheet) — verified `dialog.module.css` currently has none. Ran this file directly:
8/8 pass.

**"Also" — m9 `dismissedRecoveries` pruning coverage — CONFIRMED NOW TESTED.**
`app/src/stores/uiStore.test.ts:142-155` ("drops a closed document's dismissed recovery
banner") opens two tabs, dismisses both banners, dismisses doc 1 again (asserts no
double-count: `dismissedRecoveries` stays `[1, 2]`), then calls `closeTabsForDoc(1)` and
asserts `dismissedRecoveries` is now `[2]`. This exercises the pruning logic in
`app/src/stores/uiStore.ts` (`closeTabsForDoc` at `:313-330` filters
`dismissedRecoveries`; `dismissRecoveryBanner` at `:360-364` dedupes). Ran this file
directly: 14/14 pass. Round-1 report's overstatement is corrected.

## New Breakage in the Fix Diff

**`reset()` can silently discard an `update()` patch that was issued *after* `reset()` was
called, not just ones queued before it — Important, unaddressed.**

`app/src/stores/prefsStore.ts:142-146`:
```ts
reset: () =>
  enqueue(async () => {
    pending = undefined
    await persist({ ...DEFAULT_PREFS })
  }),
```
`pending = undefined` runs at *task-execution time* (whenever `reset`'s turn at the front
of the queue arrives), not at *call time*. `update` (`:125-126`) writes into the same
shared `pending` synchronously the instant it is called, regardless of the caller's
position in the queue. So: if `reset()` is called while an earlier write is still in
flight ahead of it in the queue, and a further `update()` call lands before `reset`'s own
task starts running, `reset`'s task wipes out that `update`'s patch — even though the
`update()` call happened, in real call order, *after* `reset()`. The patch is lost
silently: no error, no toast, no rejected promise.

Reproduced with two throwaway test files (written to `app/src/stores/`, run via `npx
vitest run`, then deleted — worktree is clean, confirmed with `git status --short`):

1. `update({theme:'dark'})`, `reset()`, `update({grid_size:8})` fired synchronously back
   to back (no `await` between them): final persisted record was `DEFAULT_PREFS` — the
   grid_size:8 patch never reached the transport. Two `prefs.set` calls occurred (the
   merged dark+grid_size write, immediately overwritten by the reset write); the third
   task for `update({grid_size:8})` found `pending` already cleared and no-opped.
2. Same three calls but with `await Promise.resolve()` (one microtask tick) inserted
   between `reset()` and the second `update()`: **same result** — `DEFAULT_PREFS`
   persisted, `grid_size:8` lost. One microtask isn't enough separation because `reset`'s
   task is still queued behind the first `update`'s task (which is doing real async work
   inside `persist`/`call`) when the second `update()` call lands and repopulates
   `pending`; by the time `reset`'s task finally gets its turn, it clears whatever is
   sitting in `pending` regardless of when that patch arrived relative to the `reset()`
   call.

This is distinct from, and not covered by, the "reset supersedes anything still queued"
design note the report itself makes (`prefsStore.ts:140-141`, and Concerns §1 in the
report) — that note describes (and the task brief accepts) `reset` discarding a patch
that was queued *before* it was called. What's demonstrated here is `reset` discarding a
patch requested *after* it was called, purely because of where its task happens to sit in
the FIFO queue at the moment the later `update()` call's synchronous `pending` write
lands. The trigger requires three rapid actions around a reset click (an update already
in flight, a reset queued behind it, then another update before reset's turn comes up) —
a real but narrower window than the original finding-2 bug, not merely a same-tick
academic case, since it survives a full microtask boundary. No test in this diff or the
existing suite exercises `reset()` interleaved with `update()` at all.

Suggested direction for a future fix: capture what `reset` is superseding at *call* time
(mirroring how `update` merges into `pending` at call time), e.g. a monotonically
increasing token per enqueued task so `reset`'s task only clears `pending` if no `update`
has been enqueued after it, or have `reset` snapshot-and-clear `pending` synchronously
when `reset()` itself is invoked rather than when its queued task runs.

No other new breakage found. Specifically checked and clean:
- Queue never wedges on a rejected write: `enqueue` uses `.then(task, task)` (both
  fulfillment and rejection continue the chain). Moot in practice today since neither
  `load` nor `persist` (`:79-94`, `:99-109`) ever reject — both catch their `call(...)`
  errors internally and push a toast instead — but the double-handler is a correct
  defensive measure, not a bug.
- No unhandled promise rejections: same reasoning; `update`/`reset`'s returned promises
  can't reject because the underlying tasks can't reject.
- `NumberField`'s Enter/blur/Escape/arrow behaviour for unbounded fields is unchanged:
  only the `clamped` expression inside `commit()` changed; algebraically, with both `min`
  and `max` undefined, both the old (`Math.min(parsed, Math.max(parsed, parsed))`) and new
  (`Math.min(Infinity, Math.max(-Infinity, parsed))`) expressions reduce to `parsed`. The
  `onBlur`/`onKeyDown` handlers themselves are untouched by the diff, and native
  `min`/`max`/`step` attributes (which drive the browser's arrow-key stepper) are also
  untouched.

## Out-of-Scope Observations

- `usePrefsStore.getState().load()` is called directly, bypassing the new queue entirely,
  from `app/src/shell/AppShell.tsx:62` (startup) and
  `app/src/dialogs/PreferencesDialog.tsx:45` (dialog mount). Either call can race an
  in-flight `update()`'s internal `await get().load()` (`prefsStore.ts:132-134`), issuing
  two concurrent `prefs.get`s that could `set()` state out of order. Pre-existing — these
  call sites and this code path are untouched by this diff — not a regression introduced
  here.
- `persist()`'s `applyToShell(prefs)` call (`prefsStore.ts:84`) sits outside the
  `try`/`catch` that wraps the `prefs.set` call. If it ever threw, `persist()`'s promise
  would reject, which — via the new queue — would still be safely absorbed by
  `enqueue`'s `.then(task, task)` for later tasks, but would surface as a rejection on
  that specific `update()`/`reset()` call's own returned promise (most call sites use
  `void update(...)`, so this would be an unhandled rejection). Pre-existing code,
  unmodified by this diff.
- `prefsStore.test.ts:152-172` ("serialises writes that arrive while an earlier one is in
  flight") is a fine test but its name overstates what it exercises: because both
  `update()` calls in the test are issued synchronously back to back (inside a
  `Promise.all([...])` array literal), it takes the same "batched into `pending` before
  the first task runs" path as the "coalesces..." test above, not a case where the second
  call genuinely arrives while the first write is mid-flight. Not a bug, just a
  documentation/naming nit.
- Report's Concerns §1 acknowledges "reset ... discards queued patches" but frames it
  only as the accepted before-reset case; it does not surface the after-reset loss window
  documented above.

## Verdict

**Fix round:** Findings remain open — all three round-1 findings and the m9 coverage gap
are correctly addressed (verified by direct inspection and by running the four
changed/added test files, 37/37 passing), but this round's queue redesign introduces a new
Important-severity, untested data-loss path: an `update()` issued while a `reset()` sits
queued behind another in-flight write can be silently discarded by that `reset()`, even
though it was requested after the reset. Needs a fix-round-3 pass on
`app/src/stores/prefsStore.ts:142-146` (and ideally a regression test for
reset-interleaved-with-update) before this can be signed off clean.
