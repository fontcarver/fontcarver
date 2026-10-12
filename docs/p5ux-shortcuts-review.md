# Keyboard usability review and repair

2026-10-02. Source audit and synthetic DOM/keyboard verification; actual browser
experience is verified separately by the parent. No native macOS claim.

Coverage: document/undo commands; palette (`Mod+K`,
`Mod+Shift+P`); numbered tabs; zoom/pan; 17 outline tools with tap/hold;
bitmap tools; selection, deletion and smooth nodes; spacing/kerning;
F1/context help; searchable, persistent, importable keymaps and presets.
Menus, palette and tool strip expose bindings. Text-canvas kerning correctly
takes priority over tab-navigation arrows.

| Priority | Reproduction and original finding                                       | Repair/source                                                                                                                                                                                 |
| -------- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P1       | Remap Pen to U, hold/release: global dispatch permanently selected Pen. | Preserve tap/hold routing and custom-binding priority: [shortcuts.ts](../app/src/shell/shortcuts.ts).                                                                                         |
| P1       | Confirm an IME candidate with Enter in the palette: command executed.   | Composition/229 guards: [CommandPaletteDialog.tsx](../app/src/shell/CommandPaletteDialog.tsx).                                                                                                |
| P1       | macOS Control+Tab failed; Command+Tab incorrectly matched.              | Explicit physical `Control`, portable legacy aliases preserved: [shortcutSyntax.ts](../app/src/shell/shortcutSyntax.ts), [workspaceTabCommands.ts](../app/src/shell/workspaceTabCommands.ts). |
| P2       | Search Pen in shortcut settings: two inconsistent rows.                 | One authoritative tool row: [keymapTargets.ts](../app/src/shell/keymapTargets.ts).                                                                                                            |
| P2       | Click/Enter-open menu, Down failed; menu actions lost canvas focus.     | Keyboard entry, dismissal and focus restoration: [MenuBar.tsx](../app/src/shell/MenuBar.tsx).                                                                                                 |

Six probes reproduced original behavior. Permanent regressions initially
reported **14 failed / 2 passed**, then **16 passed**. Additional coverage verifies
physical Control entry, storage, import/export, display and platform-specific
conflicts without rewriting saved bindings. No new UI strings or dependencies.

Focused verification: `pnpm --dir app exec vitest run` selected 14 shell/tool
files with `--maxWorkers=1`; `pnpm --dir app check`; ESLint and Prettier on all
12 changed TypeScript files; `git diff --check`.

```text
Test Files  14 passed (14)
     Tests  131 passed (131)
  Duration  13.18s
tsc -b: exit 0
ESLint: exit 0
```

Artifact checkpoints passed: build 5.33 GiB. Task-owned scratch is removed;
no servers, commits, publication or broad build/test run belonged to this task.
