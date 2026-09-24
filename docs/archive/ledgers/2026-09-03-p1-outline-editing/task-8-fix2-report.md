# Task 8 fix round 2

Status: **DONE** on `p1-t8`.

Commits:

- `c951cfd` — document-scoped edit handover, source/disposal cancellation, canvas Escape ordering, history/apply barriers, and integration regressions.
- `ebad04c` — merged current reviewed `p1-outline-editing` (`0661ce4`), including portable fixtures, compiled glyph identity, hint-warning scope, and text lookup pool fix.
- `064d5b2` — metrics resolution identity uses a hook-local document-event epoch; covers high-generation doc → low-generation doc switches.
- `7849801` — preserve rollback error reporting after disposed view callbacks are silenced.

The handover barrier is keyed by document and cleaned when closures settle. Replacement edit sessions, fields, duplicate-glyph apply, undo, and redo wait for it; cancellation’s internal `edit_end`/`undo` remains raw, preventing self-deadlock. Source identity changes capture the old document, clear aside ownership immediately, cancel live metrics/outline drags, and finish nudges. Same-source reloads retain live edits. Disposed-controller callbacks are silenced after synchronously clearing their state.

Verification:

- `pnpm --dir app gen:api` — pass, no generated diff.
- `pnpm --dir app check`, `lint`, `prettier --check .` — pass.
- `pnpm --dir app test` — 92 files, 869 tests passed.
- resolution reviewer config — 4 tests passed.
- `FC_API_PORT=8793 FC_UI_PORT=1426 pnpm --dir app e2e --workers=1` — 16 passed, optional CJK skipped because no release `fc-devserver` was built (T12-owned; fixture is present).
- Controller is 799 lines.

The actual React canvas pointer/key route covers delayed Escape cancellation followed by a command. No separate interactive browser pass was needed; user-facing design is unchanged.
