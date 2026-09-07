# T9b lookup-pool failure report — 2026-09-07

Implemented on `p1-t9b` in `e814cda fix(app): stop queued text lookups after failure`, after fast-forwarding the worktree to current P1 `6c9593c`.

`mapWithConcurrency` now records the first worker rejection, closes its shared queue immediately, lets already-started workers settle, and then rethrows that first failure. Workers therefore cannot claim further names after an ordinary lookup error. Existing `AbortSignal` checks use the same stop path, and the configured concurrency bound and result ordering remain unchanged.

The focused regression starts two delayed searches with concurrency two. The first rejects while the second is in flight; the layout rejects with that error and the test proves the remaining two queued names were never queried. Existing success, ordering, RTL, pagination, and bounded-concurrency cases remain in the same passing file.

Validation:

- `pnpm --dir app exec vitest run src/views/text/textLayout.test.ts`: 11 tests passed.
- `pnpm --dir app check`: passed after regenerating ignored API bindings from the current worktree.
- `pnpm --dir app lint`: passed.
- Prettier for both changed files and `git diff --check`: passed.

No UI, i18n, API, or generated source files changed. No merge or push performed.
