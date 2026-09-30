# P5-108 Shortcut editor and importable presets

Owner: shortcuts_help. Help and tool-key implementation are delegated within this
boundary. The shell owner mounts standalone dialogs; this task owns command
registration integration and shortcut dispatch.

## Scope and files

- `app/src/shell/shortcutSyntax.ts`, `keymap*.ts`, `ShortcutSettingsDialog*`:
  strict portable shortcut syntax, conflict review, explicit preset/import,
  searchable editor, per-command restore/unbind, JSON export.
- `app/src/stores/keymapStore.ts`: versioned local persistence; failed writes do
  not replace live bindings; imports remain drafts until Apply.
- Existing `shortcuts.ts`, menu/palette hint integration, and tool handlers:
  dispatch current bindings, preserve input/modal/native-control ownership.
- `builtinCommands.ts`: Keyboard Shortcuts opens the editor; context help command
  comes from delegated help task. Shell action registrations coordinated with
  shell_workspaces.
- English i18n keys only; independent agent translates them.

## Public interfaces

`shortcutFor(id, fallback, overrides?)` resolves explicit unbind (`null`) and
otherwise preserves existing bindings. `KeymapOverrides` maps IDs to portable
strings or null. `useKeymapStore` exposes `overrides`, `open`, `setOpen`, and
`save(overrides)`, returning success/error without mutating live state on failure.
Commands may opt into modal dispatch with `allowInModal` (help only).

## Acceptance

Write failing tests first for strict syntax, duplicate/conflicting commands,
context-exclusive existing defaults, import rejection and atomic persistence.
Test edited dispatch and hints, modifier normalization, editable/contenteditable
and native-control guards, modal help, reset/unbind and JSON round trip.
Named Glyphs/FontLab/RoboFont presets are explicit, documented supported subsets.
Focused frontend tests and isolated agent-browser visual/accessibility QA;
parent runs final broad check/lint/test after integrated changes. No commits.
