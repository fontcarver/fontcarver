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

## Gap analysis

Written 2026-10-01 after reading the code for every row. "Now" is HEAD `5bd842e`
plus wave 1. Decisions are recorded here because the protocol forbids asking.

### Row 80 — light/dark theming, canvas palettes, localisation

What works now:

- `styles/tokens.css` defines the whole UI palette under `:root` and a complete
  dark override under `:root[data-theme='dark']`; `uiStore.applyTheme` writes
  `data-theme`, `AppShell` re-applies it and subscribes to
  `prefers-color-scheme` while the preference is `system`.
- Language: `AppShell` sets `document.documentElement.lang` and calls
  `i18n.changeLanguage` on every change; `prefsStore.applyToShell` mirrors the
  kernel record into the shell, and `uiStore` also persists it in local storage.
  No reload is involved.
- Glyph and text canvases already read a _separate_ `--cv-*` palette through
  `views/glyph/glyphTheme.ts` (the text canvas draws with the same `GlyphTheme`),
  and the font grid reads UI tokens through `views/font/useCellTheme.ts`.
- No icon-only `<button>`, `<input>`, `<select>` or `<textarea>` in `app/src`
  lacks an accessible name (audited by source walk; 0 findings).

Gaps found, and what I will do:

1. **The bitmap canvas has no canvas palette.** `views/bitmap/BitmapCanvas.tsx`
   and `BitmapWaterfall.tsx` read `--color-canvas`/`--color-text`/`--color-border`/
   `--color-accent`, i.e. the UI palette. Add a `--bm-*` palette (paper, ink,
   grid, accent) to `tokens.css` for both themes and switch those two call sites
   to it (token-name-only patches in task `g`'s files).
2. **Measured contrast failures.** `--color-text-faint` on `--color-app` is
   4.27:1 in light, below the design spec's "≥ 4.5:1 for all text ≤ 14 px".
   `--cv-handle` and `--cv-component` are 2.99:1 on `--cv-bg` in light, below
   the spec's "≥ 3:1 for canvas markers". Three CSS-module rules put
   `--color-accent` text on `--color-accent-soft` (4.25:1 light / 3.56:1 dark):
   `views/features/FeaturesView.module.css:91,150` and
   `views/glyph/toolPalette.module.css:44`. Fix all of them (token value changes
   in `tokens.css`; one-line `color:` changes in the two modules, where
   `aria-pressed`/`aria-current` and the accent background already carry state).
3. **No automated guard.** Add `styles/tokens.test.ts`: light/dark token parity,
   text-contrast and canvas-marker thresholds, every `var(--…)` used by a CSS
   module exists, no literal colour outside `tokens.css`, and every CSS-module
   rule that sets both `color` and `background` meets 4.5:1 in both themes.
   Documented exemptions: the metric/sidebearing/zone/background-layer tokens are
   deliberately recessive reference strokes and alpha fills (the design spec
   itself specifies `--cv-metric` `#B8BEC7`), and
   `views/text/PreviewStrip.module.css` is a proof surface that carries its own
   `color-scheme` plus `Canvas`/`CanvasText` so the preview's light/dark state is
   independent of the UI theme. Glyph colour-label swatches
   (`views/font/colorLabels.ts` and the four `style={{ background: … }}` sites)
   are font _data_, not theme colours, and are also exempt.
4. **No English-UI toggle outside Preferences.** Row 80 asks for one. Add
   `view.language.en` / `view.language.zh-CN` commands beside the existing
   `view.theme.*` family in `builtinCommands.ts` (additive hunk), so a user who
   cannot read the current UI language can reach English from the View menu and
   the command palette.
5. **English source wording.** Apply the `docs/TODO.md` list in `en.json`:
   delete the three unused keys, normalise Licence/Colour (and "centre", the same
   class of defect — American spelling already dominates: color 22/7, license
   6/1, behavior 3/0), re-word the five `exportMatrix` strings that describe only
   named instances, align node/point and group/class wording, correct
   `fontSettings.upmHint` to the kernel's "bake or remove", and align
   `history.layerRole.smart` with `glyphInspector.layerKind.smart`. Every changed
   key is listed in the report for the translation pass. `zh-CN.json` is not
   touched.
6. **Audit evidence.** Screenshot every major surface in light and dark, EN and
   zh-CN, in the headless browser against a real kernel.

### Row 107 — Quick Help on hover (F1) and a context help panel

What works now: `contextHelp.ts` captures the focused control, the hovered
control (`pointerover`/`pointerout`, cleared on `focusin` and window blur), the
top-most non-hidden `[role="dialog"]` and the active tab kind, then opens a
modal article with sixteen offline topics, a topic picker, the dialog name and
the focused control's description. `help.context` is a registered command with
`shortcut: 'F1'`, `allowInInput` and `allowInModal`.

Gaps, and what I will do:

1. **No hover quick help.** Hovering only annotates the _next_ F1 press; nothing
   appears on hover beyond the native `title` tooltip. Add a non-modal Quick
   Help bubble that opens after a hover delay (and on keyboard focus) for a
   help-annotated control, naming the control, its command binding and its
   one-line description, with an explicit button that opens the full article at
   that topic — the "link to the context help panel article" the row asks for.
2. **No per-command help entry.** The menu maps a whole menu to one topic with a
   hard-coded switch in `MenuItem.tsx`; 146 registered commands have no help of
   their own. Add `shell/commandHelp.ts` with an explicit entry (topic + a
   one-line `quickHelp.*` summary) for every command, a total mapping with no
   fallback, and `shell/commandHelp.test.ts` enforcing that every registered
   command has an entry, that every entry names a real topic and an existing
   i18n key, and that no entry is stale.
3. **`modalTopic` is not total and misses wave-1 dialogs.** `compareFonts`,
   `pasteSpecial`, `relinkComponents` and `batchRename` fall through to the
   generic `dialog` topic; the workspace dialog, the macro console and the
   shortcut editor are not all covered. Add topics `crossDocument`,
   `batchRename`, `workspace`, `automation`, `history`, `spacing` and
   `variations`, make the dialog-kind mapping exhaustive (a `switch` over the
   union, so a new dialog kind fails to compile), and test it.
4. **The panel does not follow the active tool.** Add the active tool's name and
   gesture hint to the article and the quick-help bubble when an Edit tab is
   active.
5. **Decision (recorded limit).** The help article stays a modal overlay rather
   than becoming a docked panel: it must be able to sit above an open editing
   dialog, and its tested focus-restore contract depends on that. The
   _non-modal_ half of the requirement is served by the new Quick Help bubble.
   The article remains fully keyboard reachable (Dialog's focus trap, Escape,
   focus restore).

### Row 108 — keymap presets and a shortcut editor with conflict detection

What works now: `KEYMAP_PRESETS` carries the three documented subsets;
`keymapDocument.ts` validates a versioned, bounded JSON import and exports the
draft; `keymapConflicts` groups bindings by canonical chord and only reports a
group when the scopes actually overlap; Apply is blocked while a conflict, an
invalid binding or an unknown action id exists; `shortcutFor` makes a saved
binding win over a newly shipped default; `shortcuts.ts` dispatches from the
live overrides.

Gaps, and what I will do:

1. **Conflicts are not inline.** They are listed once in an alert at the top of
   the dialog. Mark every conflicting row inline (`aria-invalid`, a per-row
   message naming the other actions) and sort conflicting rows to the front so
   the 80-row display limit can never hide one.
2. **No explicit override.** The row allows saving "or explicitly overridden".
   Add a deliberate per-chord acknowledgement: the alert offers to allow that
   exact chord to be shared, the acknowledgement is stored with the keymap
   (`allowConflicts` in the version 1 document, ignored by older readers), and
   Apply is unblocked only for acknowledged chords. Unacknowledged conflicts
   still block.
3. **Dispatch evidence.** Verify in the headless browser that an edited binding
   dispatches, that a preset's binding dispatches, and that an unbound action
   stops dispatching.

### Accessibility sweep

1. `CloseDocumentDialog`, `PathPromptForm` and `CommandPaletteDialog` build
   their own modal markup: they handle Escape but have **no Tab focus trap, no
   focus restore and do not count themselves as modals**, so `shortcuts.ts`
   still dispatches document commands behind them (`Mod+W` behind the
   "Save before closing?" prompt). Add a small `useModalChrome` hook (my file)
   and apply it in those three (attribute/one-line patches).
2. Add `shell/a11y.test.tsx`: a source walk that fails on an icon-only button
   without an accessible name, an unlabelled field and a click handler on a
   non-interactive element, plus a rendered walk over `DialogHost`'s dialog
   kinds and the shell's own dialogs asserting an accessible name on every
   interactive element, a focus trap and Escape.
