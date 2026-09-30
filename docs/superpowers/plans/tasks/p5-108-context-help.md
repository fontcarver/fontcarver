# P5 row 108: context-sensitive help

## Scope and ownership

Implement the in-app context-help dialog, help topic selection and localized
hover/accessibility hints. Own `app/src/shell/contextHelp.ts`,
`ContextHelpDialog.tsx`, `ContextHelpArticle.tsx`, `ContextHelpDialog.module.css`, their focused tests and
`docs/user/context-help.md`. The shell/shortcut owner integrates the command,
mounts the dialog and merges the supplied English namespace. The independent
localization owner translates it. Do not change generated API types or shared
status documents.

## Public interface

- `registerHelpCommands()` registers Help > Context Help.
- `openContextHelp()` snapshots the focused dialog or current tab.
- `helpHint(topic)` supplies a localized `title`, `aria-description` and
  `data-help-topic` for annotated controls.
- `<ContextHelpDialog />` mounts once in the shell, reads its own transient
  store, allows switching topics and closes without changing the underlying UI.

F1 is registered through the shortcut dispatcher, including input fields and
modal dialogs. Help is informational and never invokes document commands.

## Acceptance

Write failing tests before implementation. Verify current-tab selection,
dialog priority/title, annotated focus, useful field hints, command registration,
help navigation, repeat-open idempotence and nested dialog focus restoration.
Run focused Vitest and file-scoped lint/format checks. Parent owns complete
frontend checks and integrated headless browser acceptance. Check artifact
budgets at start and completion and remove task scratch data after handoff.
