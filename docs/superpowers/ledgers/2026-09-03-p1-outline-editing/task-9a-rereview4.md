# Task 9a — Re-review 4 (fix round 4 verification)

Base `2bc0431` → head `dcd616a`, two commits (`2668896`, `dcd616a`). Diff read from
`review-2bc0431..dcd616a.diff`; current file contents read directly from the worktree
(`app/src/stores/prefsStore.ts`, `app/src/stores/prefsStore.test.ts`). Consumers grepped
(`PreferencesDialog.tsx`, `AppShell.tsx`, `builtinCommands.ts`). Ran
`pnpm --dir app exec vitest run src/stores/prefsStore.test.ts` (17/17 pass) and
`pnpm --dir app check` (clean) from the worktree. No git-history commands beyond the
initial `git log --oneline` sanity check; no git state mutated.

### Finding Verdict

**ADDRESSED.** The round-3 finding's root cause — one shared, anonymous `flush` draining
whatever the module-scoped `pending` buffer held at execution time, so a later call's data
could be written by an earlier call's queued task — is structurally eliminated, not just
patched around the reported scenario.

`app/src/stores/prefsStore.ts:51-56` replaces the buffer with an `Intent` captured
per-call: `update` (`:210`) and `reset` (`:220`) each call `enqueue({ kind: ..., patch })`
with the intent's own data closed over at *call* time; nothing is shared or foldable
between intents, and each `run(intent)` (`:164-184`) only ever touches the intent it was
given. There is no "whose data does this task carry" ambiguity left to exploit by timing,
because there is no shared mutable slot between enqueue and execution — the exact defect
class the round-3 finding described.

Reproduced-and-fixed check, mirroring rereview3's own probe: `update(theme:'dark')` →
`reset()` → *(microtask)* → `update(grid_size:8)`, with the reset's own write rigged to
reject. Test `builds the writes after a rejected one on what the kernel really holds`
(`prefsStore.test.ts:409-446`) asserts exactly this: `writes()` = `[stored+theme,
DEFAULT_PREFS, stored+theme+grid]` (three writes now, one per intent, not two), the reset's
write is the one that fails, the following `update(grid_size:8)` lands on `confirmed`
(`stored+theme+grid` — the *first* update's successful record) rather than either the
failed defaults or being silently discarded, `useUiStore.getState().theme` is `'dark'` (not
reverted to the default the rejected reset asked for), and exactly one error toast fires.
Ran this test: passes against `dcd616a`.

### New Breakage in the Fix Diff

None found.

Checked against every item the scope called out:

- **Queue never rejects/wedges; one toast per rejected intent; later intents run.**
  `run` (`:164-184`) catches its own `prefs.set` failure internally and pushes exactly one
  toast per failed write; `read` (`:133-144`) does the same for `prefs.get`. Because `run`
  is an `async` function, any path through it (even a hypothetical uncaught bug) resolves
  to a promise rather than throwing synchronously, and `enqueue`'s outer
  `.catch(() => undefined)` (`:190`) is a second line of defense that keeps the chain alive
  and prevents an unhandled rejection even then. Verified live: `reports a failed write
  instead of rejecting what it handed back` (`:497-520`) fires three unawaited writes
  against a transport where every `prefs.set` throws, asserts zero
  `unhandledRejection` events via `watchUnhandledRejections()`, three toasts, and that the
  promise `update()` hands back resolves (not rejects). Confirmed by running the file.
- **No deadlock between a `load` intent and an update's own first read.**
  `recordFor` (`:156-161`) calls `read()` **directly**, not `enqueue({kind:'load'})`, when
  nothing is confirmed yet — documented at `:124-125` as the deliberate reason ("calls it
  directly rather than re-entering the queue, which would deadlock"). Traced why: an
  `enqueue` call from inside a `run()` that is itself the head of the chain would append a
  new task *behind* the currently-executing one, and if that same `run()` then awaited the
  new task's promise, it would wait on a task that can only start once `run()` returns —
  circular. Calling `read()` inline sidesteps this entirely; confirmed no other code path
  re-enters `enqueue` from within `run`/`recordFor`/`read`.
- **`confirmed` moves only on success; generation stamp cannot adopt a stale reply.**
  `confirmed` is written only inside `confirm()` (`:117-121`), called only after a
  successful `prefs.get` (`read`, after the stamp check) or a successful `prefs.set`
  (`run`). Both `catch` blocks (`:139-143`, `:177-183`) touch only `prefs` (the optimistic
  display value) and never `confirmed`. The stamp (`generation`, bumped in `confirm`) is
  genuinely unreachable today, exactly as the report discloses in its own Concerns
  (`task-9a-report.md`, "The stamp guard is unreachable today"): the strict serial queue
  means no second `read()`/`run()` can start until the current one's `await`s — including
  `recordFor`'s inline `read()` — have finished, so no read is ever in flight when a
  competing confirm could land. Disclosed limitation, not a live bug.
- **Optimistic vs. confirmed state after a failure.** `run` captures
  `confirmed = get().confirmed ?? get().prefs` (`:172`) *after* `recordFor` has already
  resolved (so it reflects any read that just happened), applies the new record
  optimistically, and on rejection restores exactly that snapshot to both `prefs` and the
  shell (`:178-179`). No permanent flicker to `DEFAULT_PREFS` unless a reset's write
  actually succeeded — confirmed by the rejected-reset test above, where the shell keeps
  `theme: 'dark'` through the failed reset.
- **`reset()` semantics.** `recordFor` returns `DEFAULT_PREFS` unconditionally for a reset
  intent (`:157`, no read, no merge), and a later `update`'s `recordFor` builds
  `{ ...confirmed, ...patch }` off whatever `confirmed` was last set to — `DEFAULT_PREFS`
  after a successful reset. Matches `lets a reset asked for last undo the changes before
  it` (`:336`) and the two "change after reset" tests (`:276`, `:309`).
- **Public API unchanged.** Grepped every consumer: `PreferencesDialog.tsx:38-45` reads
  `state.prefs`/`state.loaded`/`state.update`/`state.reset` and calls
  `usePrefsStore.getState().load()`; `AppShell.tsx:62` calls `.load()`;
  `builtinCommands.ts:56` (View menu theme) calls `.update({ theme })`. None reference the
  new `confirmed` field or any changed signature — `update`/`reset`/`load` are still
  `(...) => Promise<void>` with the same parameters. No edits needed or made to any of the
  three files, matching the report's claim.
- **The 16 (now 17, +1 follow-up) tests.** `prefsStore.test.ts` has 17 `it(...)` blocks;
  ran the file, 17/17 pass. Payload order is asserted via the new `writes()` helper
  (`:13-17`) in every multi-write scenario (`:161`, `:210`, `:276`, `:309`, `:336`, `:363`,
  `:409`), the rejected-write case is `:409-446` (checked above), and the late-load race
  uses a real `setTimeout(resolve, 25)` on the mock transport's first `prefs.get`
  (`:210-255`, "keeps a slow start-up read from undoing a change made while it ran"), not
  just a microtask gap — closing the follow-up ruling's own reproduction (a slow start-up
  read installing a stale `confirmed` after a later write already landed).
- `pnpm --dir app check` (tsc -b): clean, no type errors from the `Intent`/`WriteIntent`
  discriminated unions or the `confirmed` field addition to `PrefsState`.

### Out-of-Scope Observations

- The closure-scoped `queue` and `generation` variables are not reset by
  `usePrefsStore.setState(initialPrefsState())` in the tests' `beforeEach` — only the
  exposed `prefs`/`loaded`/`confirmed` fields are. This predates this round (the `queue`
  closure existed since round 2) and is harmless in practice: every test fully awaits its
  own writes before finishing, and `generation`'s absolute value is never compared to
  anything but itself within one read's lifetime. Not introduced by, or specific to, this
  diff.
- Carried forward from the report's own Concerns, unchanged and still accurate: a burst of
  N actions is now N writes rather than one (the ruled trade-off); a rejected write is
  reported but not retried; the write queue is per store, not per process; Enter in Add
  Glyphs is reachable only from the preview list; Export falls back to its own destination
  field; the skip-reason discriminator mirrors the kernel's `is_glyph_name`; `Prefs` has no
  hit-radius or tool-defaults field.

### Verdict

**Fix round:** All findings addressed, no new Critical/Important breakage.
