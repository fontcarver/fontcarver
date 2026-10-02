# Bitmap entry focus follow-up

2026-10-02. The full browser gate found the first M shortcut ineffective after
opening Bitmap editor. Browser inspection confirmed the closed Bitmap menu
title retained focus. Global interactive-control protection correctly ignored
the letter; customized outline-tool dispatch was unrelated.

`BitmapCanvas.tsx` now focuses its newly mounted editing surface when focus is
still on the document body or a closed menu title. Deferred loading is covered.
Existing fields, buttons, open menus and modals retain ownership. Both the modal
store and mounted modal DOM are checked, covering dialogs whose registration
effect has not yet run. No menu, global shortcut, gesture or locale changes.

`BitmapEntryFocus.test.tsx` tests pointer/keyboard activation, delayed mount,
all nine bitmap tool keys, typing protection, another menu opened by hover,
and simultaneous mounting with a real Dialog. Initial regressions reported
3 failures/5 passes. The reviewer-requested Dialog timing regression failed
before the DOM guard, then passed.

Verification commands:

```sh
cd app
pnpm exec vitest run src/views/bitmap/BitmapEntryFocus.test.tsx src/views/bitmap/BitmapCanvas.test.tsx src/views/bitmap/bitmapCommands.test.ts src/shell/shortcutUx.test.tsx src/views/glyph/useToolShortcuts.test.tsx --maxWorkers=1
pnpm check
pnpm exec eslint src/views/bitmap/BitmapCanvas.tsx src/views/bitmap/BitmapEntryFocus.test.tsx
pnpm exec prettier --check src/views/bitmap/BitmapCanvas.tsx src/views/bitmap/BitmapEntryFocus.test.tsx
```

```text
Test Files  5 passed (5)
     Tests  57 passed (57)
  Duration  5.29s
tsc / ESLint / Prettier / git diff --check: exit 0
```

Independent reviewer: 28/28 passed; approved. Browser owner: original shortcut,
cancel/save and middle/Space-pan scenarios passed 3/3 without focus workarounds.
See [browser acceptance](p5ux-browser-acceptance.md) for final browser evidence.

Task scratch removed; artifact check passed (build 5.33 GiB). No commits,
publication, owned servers or broad test runs.
