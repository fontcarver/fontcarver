# P5-108 configurable tool shortcuts and compatibility presets

Scope: tool tap/hold shortcut resolution, live ToolPalette shortcut labels, and
importable Glyphs / FontLab / RoboFont shortcut subset data. Shared keymap storage,
command dispatch, shortcut editor and translations are owned by the parent task.

Owned files: `app/src/shell/toolShortcuts.ts`,
`app/src/shell/keymapPresets.ts`, their focused tests,
`app/src/views/glyph/useToolShortcuts.ts` and its test,
`app/src/views/glyph/ToolPalette.tsx` and its test,
and this task's acceptance and provenance documents.

Interfaces: `toolShortcut(tool, overrides?)` resolves `tool.<id>` through the
shared `shortcutFor`; `toolForShortcut(event, mac?)` reads current keymap state.
Presets expose `id`, `nameKey`, and `overrides`. Null disables a binding. Tool
shortcuts preserve 200 ms tap/hold behavior. Default unmodified bindings retain
Shift-insensitive fallback; explicit overrides match their exact modifiers.

Acceptance: write failing focused tests before implementation for rebinding,
unbinding, modifier matching, live updates, tap/hold, default behavior and
preventDefault cooperation. Check importable preset shape and collision-free tool
assignments. Record official source provenance and honest subset limitations.
Run focused Vitest, ESLint and Prettier checks; parent owns full frontend checks.
Check artifact budget before and after testing, remove this task's scratch output.
