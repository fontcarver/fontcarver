# Task 9a — Re-review 3 (fix round 3 verification)

Base `8cb57dd` → head `2bc0431`, one commit. Diff read from
`review-8cb57dd..2bc0431.diff`; current file contents read directly from the worktree
(`app/src/stores/prefsStore.ts`). Two throwaway test files were written to
`app/src/stores/`, run via `npx vitest run` / `pnpm --dir . exec vitest run`, then
deleted — `git status --short` confirmed clean before and after. No git-history
commands beyond the initial `git status`/`git log` sanity check; no git state mutated.

### Finding Verdict

**ADDRESSED for the two scenarios the finding described, but the fix's own mechanism
reopens the same class of bug under a third scenario the fix's tests don't cover — see
below.**

`app/src/stores/prefsStore.ts:79-84` replaces the bare `Partial<Prefs>` buffer with
`{ base: 'stored' | 'defaults'; patch: Partial<Prefs> }`. `update` (`:158-164`) folds its
patch into the existing buffer and **keeps** the current `base` (`pending?.base ??
'stored'`) instead of resetting it; `reset` (`:173-176`) replaces the whole buffer with
`{ base: 'defaults', patch: {} }`. Both mutations happen synchronously at call time, which
is exactly the ruled fix.

Verified against the ruling's two reproduction scenarios from `task-9a-rereview2.md`:
- **Back-to-back** (`update`, `reset`, `update`, no `await` between them): all three
  synchronous mutations land in `pending` before any queued `flush` runs, so the first
  `flush` to execute drains the fully-folded `{ base: 'defaults', patch: { grid_size: 8 }
  }` in one write. Confirmed by the shipped test `prefsStore.test.ts` "keeps a change
  asked for after a reset" (one `prefs.set`, `saved` = `{ ...DEFAULT_PREFS, grid_size: 8
  }`) — ran this file: 14/14 pass.
- **One microtask gap** between `reset()` and the second `update()`: still lands on
  `{ ...DEFAULT_PREFS, grid_size: 8 }` — confirmed by the shipped test "keeps a change
  asked for a tick after a reset". *However* (see New Breakage), this passes only because
  the test doesn't check the write count; my probe shows it actually costs **two**
  `prefs.set` calls, not the "single flush" the report and code comments claim, and that
  second, hidden write is what breaks under rejection.

### New Breakage in the Fix Diff

**Important — a rejected write can still make `reset()`'s effect vanish silently, via a
mechanism the four new tests never exercise.** `app/src/stores/prefsStore.ts:106-126`.

`flush` is one shared function, reused as the literal task body for every `enqueue(flush)`
call (`update` at `:163`, `reset` at `:175`). It reads and clears the *module-scoped*
`pending` at whatever moment it happens to execute (`:114-115`), not a value captured at
enqueue time. This means the correspondence between "which call enqueued this task" and
"whose patch this task actually drains" only holds when every call in a burst happens
*before* the first queued task starts running. As soon as a queued task's own write is
slow enough to overlap with a later call — which is guaranteed the instant the write
crosses even one real microtask boundary, since the mock transport's `call()`
(`app/src/test/mockTransport.ts:33-39`) always takes more ticks than a single `await
Promise.resolve()` to settle — a **later** call's data can end up drained by an
**earlier**-enqueued task, and a subsequent call's data drained by a **later** one, purely
by timing.

Concretely, for `update(theme:'dark')` → `reset()` → *(one microtask)* →
`update(grid_size:8)` (the exact "tick" scenario in the shipped test), I instrumented the
same sequence and confirmed by direct observation:

- Two `prefs.set` calls fire, not one: the first payload is bare `DEFAULT_PREFS` (the
  `reset`'s own `{ base: 'defaults', patch: {} }`, actually drained by the task `update`'s
  own call enqueued — `theme:'dark'` is silently discarded, which is correct per the
  ruling since `reset` supersedes it); the second payload is `{ ...DEFAULT_PREFS,
  grid_size: 8 }` (drained by the task `reset()`'s call enqueued, but carrying
  `update(grid_size:8)`'s data because `pending` had been refilled by the time that task's
  turn came up).
- When I made that **first** write (the one carrying the reset's empty patch) reject —
  simulating a single dropped `prefs.set`, the same class of failure the store already
  handles for a plain `update` — the **second** write, which still runs and still
  succeeds, persists `{ ...storedRecordFromBeforeAnyOfThis, grid_size: 8 }`, not
  `{ ...DEFAULT_PREFS, grid_size: 8 }`. The `reset()` call's effect is completely and
  silently gone from both the kernel and the optimistic store state: `theme` and
  `recent_limit` are back to whatever was on disk before the whole burst, as if `reset()`
  had never been called. One error toast fires ("kernel unreachable"), which a user would
  read as "my `grid_size` change might not have saved," not "your reset was undone and the
  old theme survived." The queue itself does not wedge — a further `update()` issued
  afterward still reaches the kernel correctly — so this is a silent-data-loss bug, not a
  hang.

Reproduced with two throwaway test files added to `app/src/stores/`, run, and deleted
(worktree confirmed clean via `git status --short` before and after):
1. Instrumented the shipped "tick" test to log `prefs.set` call count and payloads:
   `2` calls, payloads `[DEFAULT_PREFS, {...DEFAULT_PREFS, grid_size:8}]` — confirms the
   "single flush drains it" claim (report, "Fix round 3": *"A single `flush` (`:113-129`)
   drains it"*) does not hold once a microtask boundary separates the calls, even though
   the final state happens to still be correct when both writes succeed.
2. Same sequence with the first `prefs.set` rigged to throw once: final `saved` and final
   `usePrefsStore.getState().prefs` both equal
   `{ theme: 'light', recent_limit: 5, ..., grid_size: 8 }` — the pre-reset stored record
   plus the later patch, i.e. exactly the original finding's symptom ("an `update()`
   requested after `reset()` ... was silently destroyed" — here it's `reset()` itself that
   is silently destroyed) reappearing through a different trigger (a rejected write
   landing on the task that inherited the reset's buffer) rather than the original
   trigger (queue-position vs. call-order). A further `update({ recent_limit: 42 })`
   issued afterward still succeeded, confirming the queue does not wedge.

None of the four new tests in the diff exercise a rejected write interleaved with
`reset`/`update` (test 4, "applies a reset requested while a write is in flight," only
gates a write that later *succeeds*), so nothing in the shipped suite catches this.

**Root cause, for the fix-round-4 pass:** `flush` must not be a single anonymous function
that drains whatever `pending` happens to hold when it gets its turn — because `pending`
can be refilled by an unrelated later call between two flushes, an in-between write's
rejection rolls back to a `previous` snapshot that no longer corresponds to what the
*current* `pending` occupant assumed it was layering onto. Each enqueued task needs to
capture its own snapshot of `pending` **at enqueue time** (mirroring how `update`/`reset`
already capture intent at *call* time) rather than sharing one mutable buffer drained by
an interchangeable task; or, equivalently, a `reset`'s buffer must never be handed off to
a differently-based task for execution.

**No double `prefs.set` for a single intent in the strict sense** — the two writes in the
tick scenario correspond to two distinct buffer states (an incidental bare-`DEFAULT_PREFS`
intermediate, then the final merged one), not the same intent sent twice — but this is the
direct enabling condition for the data-loss bug above, and it contradicts the design's own
stated invariant ("A single `flush` ... drains it," "which is how several rapid actions
become a single `prefs.set`"), so it belongs in the same finding rather than as a separate
nit.

**Checked and clean, per the review's specific asks:**
- *A `stored`-base flush whose load fails must not drop later `defaults`-base intents or
  wedge the queue* — traced and confirmed fine: the `stored` branch (`:121-124`) grabs and
  clears `pending` synchronously as `flush`'s very first action, before any `await`, so a
  `reset()` call made even a tick later always lands in a **fresh** `pending` and is
  captured intact by its own queued task; a failed `load()` inside the `stored` branch
  never applies any optimistic state (the check happens before `persist` is called) and
  the queue proceeds to the next task normally. Not exercised by a test, but verified by
  code-path reading plus the general pattern confirmed live in the probes above.
- *A flush that rejects leaves the optimistic state consistent and lets later calls
  proceed* — confirmed: `persist`'s catch (`:97-103`) always rolls the store back to the
  `previous` value it captured, which does match what's actually on the kernel after a
  rejection (the store and `saved` end up equal in both my probes), and a further
  `update()` after any of this reaches the kernel normally. The optimistic state is
  internally *consistent*; the surviving bug above is that it can be consistent with the
  **wrong semantic outcome** (stored-plus-patch instead of defaults-plus-patch), not that
  it diverges from the kernel.
- `loaded` flag semantics after a `defaults` write — unchanged and fine: a successful
  `defaults` write sets `loaded: true` via `persist`'s own `set(...)` (`:96`), a failed one
  leaves `loaded` at whatever it was, and the `defaults` branch never reads or requires
  `loaded` before writing (correct — "reset means the defaults, loaded or not").
- All four ruled tests exist (`prefsStore.test.ts`, "keeps a change asked for after a
  reset," "keeps a change asked for a tick after a reset," "discards the changes a reset
  supersedes," "applies a reset requested while a write is in flight") and each asserts
  the final persisted record and/or store state exactly. Gap: the "tick" test does not
  assert `prefs.set` call count (unlike its back-to-back sibling and the "discards"
  sibling, both of which pin `toHaveLength(1)`), which is precisely where the extra,
  unadvertised second write hides.

### Out-of-Scope Observations

- The report's "Fix round 3" description overstates the mechanism slightly: "A single
  `flush` (`:113-129`) drains it" is true only for calls that are all synchronous with
  respect to each other; it is not true the moment any queued write takes long enough to
  let a later call refill `pending` before that write's own follow-on task gets its turn —
  which, per the probes above, is already true after a single microtask, not just under
  genuine network latency.
- Carried forward from round 3's own Concerns, unchanged and still accurate: the write
  queue is per store, not per process; Enter in Add Glyphs is reachable only from the
  preview list; Export falls back to its own destination field; the skip-reason
  discriminator mirrors the kernel's `is_glyph_name`; `Prefs` has no hit-radius or
  tool-defaults field.

### Verdict

**Fix round:** Findings remain open — the original finding's two literal reproductions
(back-to-back and one-microtask-gap, both with every write succeeding) are fixed and
tested, but the fix's shared-buffer/anonymous-task design reintroduces the same
silent-reset-loss failure mode under a third, untested condition (a write rejecting while
its task happens to be carrying another call's buffered intent). This needs a fix-round-4
pass on `app/src/stores/prefsStore.ts:106-126` (each queued task capturing its own
snapshot of `pending` rather than sharing one drained by an interchangeable `flush`), plus
a regression test that rigs a `prefs.set` rejection into the reset-interleaved-with-update
sequence and a call-count assertion added to the existing "tick" test, before this can be
signed off clean.
