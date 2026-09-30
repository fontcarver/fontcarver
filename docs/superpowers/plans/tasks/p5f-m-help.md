# p5f-m: Help, keymaps, theming and accessibility

Area id: `m-help`. Ports: API 8813, UI 1513. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                               | State at start                                                                                                      |
| --- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| 80  | Full light/dark theming with separate canvas palettes; localisation with an English-UI toggle             | Themes and language toggle exist; verify every surface added in P5 in both themes; canvas palettes distinct.        |
| 107 | Quick Help on hover (F1) and a context help panel                                                         | `ContextHelpDialog`, `contextHelp.ts`; see `docs/p5-context-help-acceptance.md`; hover/F1 coverage to verify.       |
| 108 | Importable keymap presets ("Glyphs", "FontLab", "RoboFont") and a shortcut editor with conflict detection | `keymapPresets.ts`, `ShortcutSettingsDialog`; see `docs/p5-shortcuts-help-acceptance.md`, `docs/keymap-presets.md`. |

Existing records: `docs/p5-context-help-acceptance.md`, `docs/p5-shortcuts-help-acceptance.md`,
`docs/p5-tool-keymaps-acceptance.md`, `docs/keyboard-shortcuts.md`, `docs/keymap-presets.md`,
`docs/p5-localization-review.md`, `app/src/styles/tokens.css`.

## Scope notes

- Row 80: audit every dialog, panel and view in light and dark (screenshots of
  each major surface in both themes, EN and zh-CN); fix contrast and token
  misuse (no hard-coded colours outside `tokens.css`); canvas palettes for
  glyph, bitmap and text canvases differ from the UI palette and are
  theme-aware; the language toggle switches the UI live without reload and
  persists; `lang` attribute follows.
- Row 107: F1 (and a hover delay) shows quick help for the focused/hovered
  control with a link to the context help panel article; every command in the
  registry has a help entry (test that enforces it); the panel follows the
  active view/tool and is keyboard reachable.
- Row 108: import the three named presets (and a file), the editor shows
  conflicts (same chord, same context) inline and blocks saving until resolved
  or explicitly overridden, exports the current keymap, and dispatch honours
  the edited bindings (browser evidence); documented in `docs/keymap-presets.md`.
- Accessibility sweep: every interactive element keyboard reachable, icon
  buttons have `aria-label`, focus order sane in dialogs, focus trap and Escape
  in every dialog; add an automated check (e.g. a Vitest that walks the
  rendered command registry/dialog host for missing labels).

## Owned files

`app/src/shell/{ContextHelpArticle.tsx,ContextHelpDialog*,contextHelp.ts,keymapDocument.ts,keymapPresets.ts,keymapTargets.ts,keymap.ts,ShortcutSettingsDialog*,ShortcutSettingsForm.tsx,shortcuts.ts,shortcutSyntax.ts,toolShortcuts.ts}` and tests;
`app/src/stores/keymapStore.ts`; `app/src/styles/**`; `app/src/i18n/{index.ts,i18n.ts}` (whatever holds the language toggle, not the catalogues);
`docs/keyboard-shortcuts.md`, `docs/keymap-presets.md`; `docs/p5f-m-help-acceptance.md`;
`docs/screenshots/p5f-m-help/**`; user docs for help and shortcuts.

Shared files (narrow patches only): any `*.module.css` for contrast/token
fixes (one-line changes, re-read first), component files for missing
`aria-label`s (attribute-only changes), `app/src/shell/{builtinCommands,commands}.ts`,
`app/src/dialogs/{DialogHost,PreferencesDialog}.tsx`, `app/src/i18n/en.json`.
