# Task 9b headless text E2E report

Date: 2026-09-07  
Model: gpt-5.6-sol  
Worktree: `.worktrees/p1-text-ui`  
Commit: `742f39c` (`app/e2e/text.spec.ts` only)

## Evidence

Focused Playwright ran headlessly with one worker and isolated ports:

```text
FC_API_PORT=8794 FC_UI_PORT=1427 pnpm exec playwright test e2e/text.spec.ts --workers=1 --reporter=list
1 passed (5.1s); test body 4.1s
```

The test opens the real DejaVu fixture, enters Text mode through the glyph canvas, types `Hamburgefonstiv`, and checks the run count and exact first/second shaped origins against `text.shape`. It performs an actual pointer click on the third run glyph (`m`), confirms the correct document gid, drags an on-curve point through the active glyph's local viewport transform, and verifies the persisted API outline.

Three real `Control+ArrowRight` presses produce width `+3`; after the 500 ms close, one undo restores metrics while retaining the pointer edit, and the next undo restores the point. Two Escape presses clear selection then return to Text mode. Changing preview size to 256 verifies editor scale `256 / 2048`.

Prettier, TypeScript check, and scoped ESLint pass. The first browser run exposed a StrictMode lifecycle bug that disposed the shaping coordinator and left zero glyphs; Astra fixed it with a StrictMode regression before the passing rerun.

This is focused preliminary evidence. The final integrated headless suite still requires rerun after remaining T9 integration.
