# P2a 08: tab handover barriers

## Scope and ownership

`app/src/shell/TabBar.tsx`, `tabActions.ts`, their tests, and the tab cycling
calls in `builtinCommands.ts`. Keep document close confirmation in the existing
session store. Root also authorized the necessary session open/create/recover
callers, export warning navigation, and an optional `activate` flag on
`uiStore.openTab` to retain obsolete open results in the background.
After review, root added `editor/editSession.ts` edit-intent revisions so a new
run started during an earlier closure causes another settlement pass. The text
feature owner marks staged number drafts with the same revision helper.

## Interfaces

- `selectTab(id)` waits for the old editor and pending document edits, then
  returns whether the requested tab became active.
- Glyph and Font Info opening and active text-tab closing use the same gate.
- Delayed requests check the active tab, editor and document before applying.
  A newer navigation request cancels an older pending one.
- Keyboard focus follows successful selection, after edits have settled.
- `prepareTabTransition()` captures the old editor and provides a guarded
  settlement pass for delayed document opens and export-warning lookups.
- `documentEditRevision(doc)` and `noteDocumentEdit(doc)` let settlement repeat
  until no new edit intent was staged while awaiting previous commands.

## Acceptance

Write delayed handover tests before implementation: document switch, overlapping
requests, changed editor, failed settlement, active text-tab close, and opening
glyph/Font Info tabs. Cover keyboard focus and command tab cycling. Run focused
Vitest checks, then `pnpm check`, `pnpm lint`, and `pnpm test`. Report unrelated
failures without changing other owners' files. No native desktop automation.

## Progress

- [x] Delayed handover tests fail on the previous synchronous implementation.
- [x] Tab actions and tab strip await settled edits.
- [x] Focus and command cycling use guarded selection.
- [x] Focused and integrated frontend verification recorded.

## Verification log

- Initial regressions: all 8 tab barriers and all 4 session-open tests failed
  against the previous immediate transition behavior.
- Focused revision regression run: 19 tests passed across `tabBarriers`,
  `sessionTabBarriers`, and `editSession`, including a real kerning run begun
  while an older edit closes and a delayed server result.
- Initial integrated frontend run: `pnpm check` and `pnpm lint` passed;
  `pnpm test` passed 988 of 989 tests, with the only failure a translation key
  concurrently being added by the translation owner.
- Final scoped work check (test runner start time 01:25:59):
  `pnpm check && pnpm lint && pnpm test` passed. Final output:

  ```text
  Test Files  114 passed (114)
       Tests  994 passed (994)
    Duration  56.06s
  ```

  Root also owns the consolidated milestone run after all agents finish.
