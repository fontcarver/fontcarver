# P5f task m — help, keymaps, theming and accessibility

Area `m-help`, rows 80, 107 and 108 of [the feature audit](p5-feature-audit.md).
Scope, ownership and the gap analysis this work started from are in
[the task file](superpowers/plans/tasks/p5f-m-help.md). Ports: API 8813, UI 1513.
Date: 2026-10-01. Branch `p5-color-authoring` on top of `5bd842e`, in the shared
wave-2 checkout with five other area agents working at the same time.

## Summary per row

| Row | Recommendation | What carries it                                                                                                                                                                                                   |
| --- | -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 80  | Complete       | Three distinct, theme-aware palettes; measured contrast in both themes with a guard; live, persisted language switch reachable from the View menu; the English wording list from TODO applied.                    |
| 107 | Complete       | Quick Help on hover and on focus with a link into the article; a help entry for every one of the 163 registered commands, enforced by a test; the article follows the view, the dialog, the command and the tool. |
| 108 | Complete       | Inline per-row conflicts, an explicit and persisted override, presets, JSON import/export, and browser evidence that an edited binding dispatches.                                                                |

Limits that remain are listed under [Limits](#limits); none of them is a missing
part of the three rows' ordinary workflow.

## Row 80 — theming and localisation

### Three palettes, both themes

`app/src/styles/tokens.css` now carries three separate palettes, each defined for
light and for dark:

- the interface palette (`--color-*`), used by the chrome and by the font
  overview's cell renderer;
- the glyph-canvas palette (`--cv-*`), read by `views/glyph/glyphTheme.ts` and
  shared by the shaped-text canvas, which draws with the same `GlyphTheme`;
- a new pixel-canvas palette (`--bm-*`), added by this task. Before it, the
  strike editor and the bitmap waterfall painted with `--color-canvas` and
  `--color-text`, so the pixel surface was the panel surface.

Measured in the running application (`getImageData` on the real canvases, with
the token values read from the document element beside them):

| Surface                | Theme | Painted ground     | Token                  | Interface ground           |
| ---------------------- | ----- | ------------------ | ---------------------- | -------------------------- |
| Glyph canvas           | dark  | `rgb(26,29,33)`    | `--cv-bg` `#1a1d21`    | `--color-canvas` `#1a1a1e` |
| Glyph canvas           | light | `rgb(255,255,255)` | `--cv-bg` `#ffffff`    | `#ffffff`                  |
| Bitmap pixel canvas    | light | `rgb(253,253,251)` | `--bm-paper` `#fdfdfb` | `#ffffff`                  |
| Bitmap pixel canvas    | dark  | `rgb(21,24,29)`    | `--bm-paper` `#15181d` | `#1a1a1e`                  |
| Bitmap waterfall strip | both  | same as above      | `--bm-paper`           | —                          |

`src/styles/tokens.test.ts` keeps the three apart: it fails if a canvas renderer
resolves a `--color-*` token for its ground or its ink (the accent and the
hairline are the two deliberate shared values, and the test says so), and if
`--bm-paper` or `--bm-ink` ever equals its interface counterpart. The glyph
canvas's light ground is white, which is the same value the interface uses for a
field; that is the design spec's own choice (`--cv-bg` light `#FFFFFF`), and the
point of the separate token is that the two move independently, as the dark row
of the table shows.

### Measured contrast, and the three defects it found

The design spec (`docs/design/editor-ui.md` §12) asks for ≥ 4.5:1 for text ≤ 14 px
in both themes and ≥ 3:1 for canvas markers against `--cv-bg`. Two checks enforce
it now.

1. **Token-level**, in `src/styles/tokens.test.ts`: every text token against every
   surface ground, the accent/danger/warning inks against the surfaces and their
   own soft chips, every canvas marker against `--cv-bg`, every `--bm-*` marker
   against `--bm-paper`, plus a classification test that fails when a new `--cv-*`
   token is neither a marker nor on the documented recessive list, and a sweep of
   every CSS-module rule that sets both `color` and `background`.
2. **Runtime**, in the browser: a script walks every element that owns visible
   text, composites its color over its effective background, and reports any pair
   below 4.5:1 (3:1 for large or bold text). It ran over each surface in light and
   dark, in English and in Simplified Chinese.

Defects found and fixed:

- `--color-text-faint` light was 4.27:1 on `--color-app`; now `#64646b` (4.75:1).
- `--cv-handle` and `--cv-component` light were 2.99:1 on `--cv-bg`; now `#8b939f`
  (3.10:1), a change invisible at 1 px stroke width.
- `--color-accent` **dark** was 4.16:1 on `--color-surface-raised`. The runtime
  audit found it on the Quick Help bubble's own link, which the token-level list
  had not predicted. The dark accent is now the design spec's `#7fa6dc` (5.39:1
  raised, 6.02:1 surface), with `--color-accent-hover` lifted to `#93b6e6` so the
  hover state stays lighter than the accent, as it must in a dark theme.
- Three CSS-module rules put accent text on `--color-accent-soft` (4.25:1 light,
  3.56:1 dark): `views/features/FeaturesView.module.css:91,150` and
  `views/glyph/toolPalette.module.css:44`. They now use `--color-text` (14.3:1 /
  9.4:1); the accent background and `aria-pressed`/`aria-current` still carry the
  state, so nothing is lost but the contrast failure.
- `views/font/glyphTableHeader.module.css` referenced `--shadow-menu` (undefined,
  with a literal fallback) and `--text-weight-regular` (undefined, no fallback, so
  the declaration was silently dropped); now `--shadow-popover` and `normal`.
- `views/font/opticalSpacing.module.css` referenced `--font-weight-medium`, which
  does not exist — the correct token is `--text-weight-medium`; the table header
  was rendering at the inherited weight.
- `views/hints/hints.module.css` had `stroke: var(--color-warning, #c87b1a)`, a
  literal color outside `tokens.css`; the fallback is removed.

**Documented exemptions**, encoded in the test:

- `--cv-metric`, `--cv-sidebearing`, `--cv-zone`, `--cv-background-layer` and
  `--cv-ruler` are deliberately recessive: they are reference strokes and alpha
  washes behind the drawing. The design spec fixes `--cv-metric` at
  `#B8BEC7`/`#3F4750` itself and describes the background layer (60 % alpha) and
  the zones (30 % alpha) as washes. Raising them would put the reference lines in
  front of the outline being edited.
- `views/text/PreviewStrip.module.css` is the only stylesheet allowed literal
  colors. It is a proof surface with its own light/dark switch, independent of
  the interface theme, so it sets `color-scheme` and paints with `Canvas`,
  `CanvasText` and `currentcolor` rather than the interface palette. That is
  correct for a proof and is now recorded in the test's allowlist rather than
  silently tolerated.
- The twelve glyph color-label swatches (`views/font/colorLabels.ts` and the four
  `style={{ background: … }}` sites) are font _data_, not theme colors.

### Runtime contrast audit results

Every number below is "elements with visible text checked, failures". All passes
were run against the real kernel on ports 8813/1513 with DejaVu Sans open.

| Surface                                  | light EN | dark EN                                              | light zh-CN | dark zh-CN |
| ---------------------------------------- | -------- | ---------------------------------------------------- | ----------- | ---------- |
| Font overview (grid, sidebar, inspector) | 461, 0   | 461, 0                                               | 461, 0      | 461, 0     |
| Edit tab (canvas, text bar, preview)     | 166, 0   | 288, 0                                               | —           | —          |
| Font Info                                | 130, 0   | 130, 0                                               | —           | —          |
| Features tab                             | 130, 0   | 131, 0                                               | —           | —          |
| Strike editor (with a 16 px strike)      | 295, 0   | 290, 0                                               | —           | —          |
| Export                                   | 201, 0   | 201, 0                                               | 532, 0      | 532, 0     |
| Find problems                            | 137, 0   | 137, 0                                               | 468, 0      | 468, 0     |
| Compare fonts / family                   | 145, 0   | 145, 0                                               | 476, 0      | 476, 0     |
| Preferences                              | 140, 0   | 140, 0                                               | 471, 0      | 471, 0     |
| Add glyphs                               | 170, 0   | 170, 0                                               | 501, 0      | 501, 0     |
| Saved workspaces                         | 137, 0   | 137, 0                                               | 468, 0      | 468, 0     |
| Scripts & plugins                        | 156, 0   | 156, 0                                               | 487, 0      | 487, 0     |
| Keyboard shortcuts editor                | 274, 0   | 382, 0                                               | 713, 0      | 713, 0     |
| Keyboard shortcuts with a live conflict  | 489, 0   | —                                                    | —           | —          |
| Context help article                     | 172, 0   | 144, 0                                               | 475, 0      | 475, 0     |
| Command palette (every command listed)   | 553, 0   | 1,169, 0                                             | —           | —          |
| Quick Help bubble over a palette row     | —        | 1,174, 0 (after the accent fix; 1 failure before it) | —           | —          |

The zh-CN counts are higher because those passes were run with the dialog over
the font overview rather than over Font Info; the dialog's own elements are
included in both.

### Light/dark and EN/zh-CN screenshots

[`docs/screenshots/p5f-m-help/`](screenshots/p5f-m-help/): `overview-light-en`,
`overview-dark-en`, `overview-light-zh`, `overview-dark-zh`, `edit-light-en`,
`edit-dark-en`, `fontinfo-dark-en`, `export-dark-en`, `bitmap-light-en`,
`bitmap-dark-en`, `shortcuts-dark-en`, `shortcuts-conflict-light-en`,
`shortcuts-allowed-light-en`, `help-article-light-zh`, `help-article-dark-zh`,
`help-article-command-light-en`, `quickhelp-menu-light-en`,
`quickhelp-palette-light-en`, `quickhelp-palette-dark-en`, `view-menu-light-en`,
`dispatch-edited-binding-light-en`. Each is a 1440×1000 capture under 300 KB.

### Language

`view.language.en` and `view.language.zh-CN` are new commands beside the existing
`view.theme.*` family, so the interface language is reachable from the View menu
and the command palette as well as from Preferences — a reader who cannot read the
current language no longer has to find the right row of a dialog. Both routes
write the same kernel preference through `prefsStore`, which mirrors it into the
shell; the command is disabled, with a reason, for the language already in use.

Verified in the browser: choosing **中文界面** switched `<html lang>` to `zh-CN`
and the whole menu bar to Chinese **without a reload**; a subsequent full reload
came back as `zh-CN`, so the choice persists. Switching back to English from the
Chinese menu works the same way.

### English source wording

Applied the list recorded in [TODO](TODO.md) (from
[pass 1](p5f-localization-pass1.md) and [pass 2](p5f-localization-pass2.md)).
Removed, having confirmed no `t()` reference in `app/src` or `app/e2e`:
`exportProfiles.hint`, `history.font`, `history.current`. Changed:

| Key                                                                                                                                                | Why                                                                                                                                                                                                              |
| -------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `plugins.provenance`                                                                                                                               | "Licence" → "License"                                                                                                                                                                                            |
| `sidebar.colors`, `fontView.color.unknown`, `fontView.colorLabelGroup`, `glyphInspector.color`, `editor.undo.setColorLabel`, `panel.glyph.noColor` | "Colour" → "Color"                                                                                                                                                                                               |
| `editor.undo.align.centerH`, `panel.transform.align.centerH`, `panel.transform.align.middle`, `infoBox.originPoint.1`, `infoBox.originPoint.7`     | "centre" → "center", the same class of defect; American spelling already dominated (color 22/7, license 6/1, behavior 3/0)                                                                                       |
| `fontSettings.upmHint`                                                                                                                             | now says what the kernel says — layer effects, constructions, linked nodes, guide-owned nodes and color outlines must be _baked or removed_, MATH and SVG _removed or re-authored_ (`handlers/variation.rs:406`) |
| `exportMatrix.loading`, `profiles`, `noInstances`, `changed`, `error.invalid-instance`                                                             | rows are named instances _and_ variable-font configurations, and the profiles are export profiles, not "compiled-font profiles"                                                                                  |
| `editor.audit.detail.coincidentManual`                                                                                                             | "points" → "nodes", matching `coincident`                                                                                                                                                                        |
| `kerningTable.classRule`, `kerningTable.compressHint`                                                                                              | "Class"/"class rules" → "Group"/"group rules", matching `kinds.groups`                                                                                                                                           |
| `history.layerRole.smart`                                                                                                                          | "Smart layer" → "Smart source", the product term used by `glyphInspector.layerKind.smart` and `docs/user/variable-and-cjk.md`                                                                                    |
| `shortcuts.conflict`                                                                                                                               | now also offers the deliberate override                                                                                                                                                                          |

Two Vitest expectations that asserted the old English were updated with them:
`views/font/fontCommands.test.ts:138` and `views/font/GlyphGrid.test.tsx:445,456`.

`app/src/i18n/zh-CN.json` was **not** touched. `src/i18n/i18n.test.ts` fails, as
the protocol expects: the Chinese catalogue is 3,523 keys against 3,924 English
ones after the whole wave's additions.

## Row 107 — Quick Help on hover (F1) and the context help panel

### Quick Help

A non-modal bubble (`shell/QuickHelp.tsx`) opens after a 600 ms hover **or
keyboard focus** on any control that carries a help annotation, and shows:

- the control's name, or the title of the command it runs;
- one line saying what it does;
- the command's **live** binding, read through the keymap overrides;
- why the command is unavailable, when it is;
- **Open help article**, which opens the full article at that control's topic;
- the current F1 binding, or, when F1 is unbound, the Help-menu route.

It never takes focus, is dismissed by a pointer press, any key, a focus change or
the pointer leaving, and stands down for a control an open modal has covered and
while the article itself is open. `helpHint` no longer emits a `title`, so there
is no native tooltip racing the bubble; the screen-reader description stays on
`aria-description`.

Browser evidence (`quickhelp-menu-light-en.png`): hovering **File → Export…**
raised "Export… / Opens Export, which compiles a font file for other applications
and reports its diagnostics. / Shortcut Ctrl+E / Open help article / Press F1 for
the full article, or Escape to dismiss." Pressing F1 from there opened the article
at the `export` topic (`help-article-command-light-en.png`), with the command
block at the top and `Exporting a font` selected in the topic picker.

### A help entry for every command

`shell/commandHelp.ts` holds an explicit entry — an article topic and a
`quickHelp.*` one-line description — for every registered command. The mapping is
total: there is no fallback, and `shell/commandHelp.test.ts` fails when

- a registered command has no entry (it caught three families during this task as
  other agents registered them: the nine strike-editor pixel tools and four
  overlays, `view.glyph.verticalMode`, and `tool.magnet`);
- an entry names a topic that is not shipped;
- an entry's summary key is missing from the English catalogue;
- an entry exists for a command that no longer does;
- a registered command's title or `disabledReason` is not in the catalogue.

The registry currently holds **163 commands**; 140 explicit entries cover them,
the difference being five families whose members share one description (the twelve
color labels, the nine numbered tab shortcuts, the three theme choices, the two
language choices and the two text-kerning gestures).

The menu rows and the command-palette rows now carry `data-help-command`, so a
command gets its help on every surface it appears on without per-surface work.
`MenuItem` no longer maps a whole menu to one topic through a hard-coded switch.

### The panel follows the view, the dialog, the command and the tool

- Seven new articles cover the surfaces wave 1 added: `history`, `spacing`,
  `variations`, `crossDocument`, `batchRename`, `workspace` and `automation`
  (23 topics in all, each with a title, a summary and three steps).
- `dialogTopic` is a `switch` over the `DialogState` union, so a new dialog kind
  fails to compile rather than falling through to the generic "Working in a
  dialog" article. `compareFonts`, `pasteSpecial`, `relinkComponents` and
  `batchRename` used to fall through; the workspace manager and the script
  console are covered too. A table test asserts that each of the thirteen dialog
  kinds maps to a shipped article and never to `dialog`.
- The article names the **active tool** and its gesture hint while an Edit tab is
  active and not in shaped-text mode — the same condition `useToolShortcuts` uses
  to decide whether a tool key is live at all, so the help never names a tool that
  the keyboard cannot reach. A borrowed tool wins over the selected one.
- When a control's own `aria-description` is the command summary, the article no
  longer prints it twice (found in the browser, fixed, test added).

### Decision recorded as a limit

The full article stays a **modal overlay** rather than becoming a docked panel.
It has to be able to sit above an open editing dialog — that is how F1 works
inside Export or Preferences — and its tested contract (the underlying dialog
keeps its settings, Escape returns focus to the control you came from) depends on
it. The non-modal half of the row is served by the Quick Help bubble, which is
new in this task. The article is fully keyboard reachable: focus trap, Escape,
focus restore, and a `<select>` topic picker.

## Row 108 — presets and the shortcut editor

### Conflicts, inline and overridable

- Every conflicting row is marked where it is edited: `aria-invalid` on the
  field, a red border, and a message naming the other actions that share the
  chord (`shortcuts-conflict-light-en.png`).
- Conflicting rows are sorted to the front of the list, so the 80-row display
  limit can never hide the row that blocks a save. A regression test registers
  120 filler actions and asserts both conflicting rows are still rendered.
- Apply stays disabled while a conflict is unresolved.
- **Allow &lt;chord&gt; to be shared** records the conflict as deliberate; Apply is
  then enabled, the rows stay marked in the warning color, and the acceptance is
  stored with the keymap as `allowConflicts` and exported with it
  (`shortcuts-allowed-light-en.png`). An older reader ignores the field and still
  applies the bindings, so the document stays at version 1. **Stop sharing** takes
  it back and blocks Apply again, and an acceptance whose conflict was resolved is
  not kept.
- Bindings that are exclusive by context are not conflicts. The strike editor's
  single-key pixel tools (B, E, L, R, O, G, M, I, H) landed in this checkout
  during the task and were reported as conflicting with the outline tools, because
  `keymapTargets.scopeOf` put them in the global scope. They are live only while a
  Bitmap tab is mounted, exactly as the glyph tools are live only on an Edit tab,
  so they are scoped `bitmap` now and `keymapIntegration.test.ts` is green again.

Live evidence, through the real editor on port 1513:

```text
field file.open := Mod+S
  aria-invalid    true
  row message     "Ctrl+S is also bound to Save."
  summary         "Ctrl+S conflicts: Open Font…, Save. Change or clear one of
                   these bindings, or allow the conflict deliberately."
  Apply           disabled
after "Allow Ctrl+S to be shared"
  aria-invalid    false
  row message     "Ctrl+S is deliberately shared with Save."
  status          "The conflict is allowed in this draft. Apply to keep it."
  Apply           enabled
after Apply, localStorage fontcarver.keymap.v1
  {"version":1,"bindings":{"file.open":"Mod+S"},"allowConflicts":["Mod+S"]}
```

### Dispatch honours the edited bindings

With that map saved, pressing **Ctrl+S** in the running application opened the
**Open Font** prompt, not Save (`dispatch-edited-binding-light-en.png`): the
edited binding dispatches, and a saved binding still wins over a shipped default
on the same chord. Loading the **Glyphs** preset and applying it wrote the
documented fifteen-binding subset to storage, dropped the now-stale acceptance,
and the tool strip relabelled itself live — the Knife button's tooltip read
"Knife (E)".

A bare tool letter is deliberately _not_ dispatched while an Edit tab is in
shaped-text mode, where the letter is typed into the run; that is
`commandForEvent`'s documented text-mode rule and `useToolShortcuts`' own
enablement condition. The tool-key path itself is covered by
`src/shell/toolShortcuts.test.ts` and `src/views/glyph/useToolShortcuts.test.tsx`.

## Accessibility

### A leak found and fixed

Three dialogs build their own modal markup instead of using the `Dialog`
primitive — the close prompt, the path prompt and the command palette. None of
them counted itself as a modal, so the global shortcut dispatcher kept running
behind them: `Mod+W` behind a "save before closing?" prompt could close the very
document the prompt was asking about. The close prompt and the path prompt also
had no Tab trap and no focus restore.

`shell/useModalChrome.ts` now supplies all four behaviours and is spread onto the
dialog element. The close prompt's body moved into `shell/CloseDocumentPrompt.tsx`
so it is mounted only while it is on screen — a first attempt kept the hook in the
always-mounted host, which incremented the modal count permanently and stood the
whole dispatcher down. **That regression was caught in the browser, not by the
tests**: `Control+1` stopped toggling the sidebar while F1 still worked, which is
the signature of a leaked modal count. The fix is the on-demand mount; the test
now asserts the count is 0 before the prompt arrives and 1 after, and the live
application was re-checked (`Control+1` prevented and the sidebar toggled).

The chrome captures the element to restore focus to during the first render
rather than from an effect, because React applies an `autoFocus` control during
the commit: read from an effect, "restore" would target a node inside the dialog
that is about to be removed. The close prompt autofocuses Save, which is exactly
that case.

### The automated check

`shell/a11y.test.tsx` has two halves.

A source walk over every `.tsx` in `app/src`, which fails on

- an icon-only `<button>` with no accessible name (children that are all
  `aria-hidden` count as none);
- an `<input>`, `<select>` or `<textarea>` with no label, `id` or `aria-label`
  that is not inside a `<label>`/`<Field>` — a field made unreachable with
  `tabIndex={-1}` behind its own labelled button is exempt and the test says so;
- an `onClick` on a non-interactive element with no keyboard path;
- a hand-built `role="dialog"` without `aria-modal`, an accessible name and key
  handling.

The walk was checked against deliberate regressions: a bare icon button, an
unlabelled input and a clickable `<div>` planted in `EmptyState.tsx` were each
reported with their file and line, and the file was restored.

A rendered walk over the close prompt, the path prompt, the shortcut editor and
the help article, asserting an accessible name on every focusable control, the
modal count, a Tab trap that cannot leave the dialog, Escape, and focus restore.

The source walk found **no** unlabelled icon button, unlabelled field or
keyboard-unreachable click target in `app/src` when it was first run, which is why
it is written as a guard rather than a fix list. Its one exemption was added for a
real pattern it flagged afterwards: `views/text/ProofLibraryPanel.tsx`'s hidden
file input, which carries `tabIndex={-1}` and is driven by its own labelled Import
button.

## Commands run

```text
$ TMPDIR=.../.scratch/p5f-m-help/tmp pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
    src/shell src/styles src/views/font/fontCommands.test.ts src/views/font/GlyphGrid.test.tsx
Test Files  34 passed (34)
Tests       277 passed (277)

$ pnpm --dir app exec eslint src/shell src/styles src/stores/keymapStore.ts src/i18n
[exit 0, no output]

$ pnpm --dir app exec prettier --write <the 35 files this task touched>
[all formatted]

$ pnpm --dir app check            # tsc -b
Errors remain only in other areas' in-flight files (ExportOptions.tt_autohint from
the hinting task, the bitmap grid preset, the proofing preview settings). No error
in src/shell, src/styles, src/stores/keymapStore.ts or src/i18n.

$ pnpm --dir app exec vitest run src/i18n/i18n.test.ts
Tests  2 failed — expected: zh-CN holds 3,523 of the 3,924 English keys after the
whole wave's additions. The translation pass follows the wave.

$ flock .scratch/p5f/cargo.lock cargo build -p fc-devserver
Finished `dev` profile in 54.54s   (two earlier attempts failed inside fc-formats
while the hinting task was mid-edit; retried per the protocol)
```

Browser runs used `target/debug/fc-devserver --port 8813 --open
fixtures/fonts/DejaVuSans.ttf` with Vite on 1513 and the `p5f-m-help`
agent-browser session. All three were stopped at the end of the task, and
`.scratch/p5f-m-help/` was removed.

```text
$ python3 scripts/check_artifacts.py      # final
build: 6.54 GiB, 15,618 files
temporary: 0.11 GiB, 1,076 files
workspace free: 625.32 GiB
PASS
```

## New i18n keys

All in `app/src/i18n/en.json`; `zh-CN.json` untouched.

- `quickHelp.*` — **140 keys**, one line per command, under `file`, `edit`,
  `font`, `glyph`, `path`, `layer`, `view`, `window`, `workspace`, `palette`,
  `help`, `text`, `tool`, `bitmap`, `automation`.
- `contextHelp.topics.{history,spacing,variations,crossDocument,batchRename,workspace,automation}`
  — 7 new articles, 35 keys.
- `contextHelp.{quickHelp,quickHelpHint,quickHelpHintNoKey,openArticle,activeTool,activeCommand,binding,unbound,unavailableHere}`
  — 9 keys for the new chrome.
- `shortcuts.{conflictAllowed,allowConflict,rejectConflict,conflictAccepted,rowConflict,rowConflictAllowed}`
  — 6 keys for the inline conflicts and the override; `shortcuts.conflict` reworded.
- `cmd.view.language.{en,zh-CN}` and `cmd.reason.languageActive` — 3 keys.

Plus the 23 corrected and 3 removed keys listed under row 80.

## Files

Owned and changed: `app/src/shell/{contextHelp.ts,ContextHelpArticle.tsx,ContextHelpDialog.tsx,ContextHelpDialog.module.css,commandHelp.ts,commandHelp.test.ts,QuickHelp.tsx,QuickHelp.module.css,quickHelp.test.tsx,a11y.test.tsx,useModalChrome.ts,CloseDocumentPrompt.tsx,keymap.ts,keymapDocument.ts,keymapTargets.ts,ShortcutSettingsForm.tsx,ShortcutSettingsDialog.module.css,ShortcutSettingsDialog.test.tsx}`,
`app/src/stores/keymapStore.ts`, `app/src/styles/{tokens.css,tokens.test.ts}`,
`docs/{keyboard-shortcuts.md,keymap-presets.md}`,
`docs/user/{context-help.md,appearance.md}`, this file and
`docs/screenshots/p5f-m-help/**`.

Shared files, narrow patches only:
`app/src/i18n/en.json` (the key list above), `app/src/shell/builtinCommands.ts`
(the additive `languageCommand` family), `app/src/shell/MenuItem.tsx` and
`CommandPaletteDialog.tsx` (per-command help markers; the palette also counts
itself as a modal), `app/src/shell/CloseDocumentDialog.tsx` and
`PathPromptForm.tsx` (modal chrome), `app/src/views/bitmap/{BitmapCanvas.tsx,BitmapWaterfall.tsx}`
(four and two token names), `app/src/views/features/FeaturesView.module.css`,
`app/src/views/glyph/toolPalette.module.css`,
`app/src/views/font/{glyphTableHeader.module.css,opticalSpacing.module.css}`,
`app/src/views/hints/hints.module.css` (one-line contrast and token fixes), and
`app/src/views/font/{fontCommands.test.ts,GlyphGrid.test.tsx}` (the two English
strings this task corrected).

No new dependency, no FontForge code, no commit, and no edit to
`p5-feature-audit.md`, `STATUS.md`, `TODO.md` or `progress-estimate.md`.

## Limits

1. **The help article is modal, not docked.** Reasoned above; the non-modal layer
   is the Quick Help bubble.
2. **Quick Help does not open for a disabled menu item**, because a disabled
   button receives no pointer events. Those rows already print their reason
   inline and expose it as `aria-description`, so nothing is hidden.
3. **Contrast is measured, not simulated for colour vision.** The design spec's
   rule that colour is never the only carrier of state is met by construction
   (node shapes, `aria-pressed`, icons on warnings) but is not machine-checked.
4. **The runtime audit covers the surfaces this task could reach** with DejaVu
   Sans open: the seventeen in the table above. Surfaces that need data this
   fixture does not have — the variable-font design-space views, the CJK
   construction grids, the colour/COLR editors, proof-sheet export — are covered
   only by the token-level guard, which is a check of the stylesheets rather than
   of the rendered result.
5. **`--color-active`, `--color-sunken` and the soft chips are not in the
   token-level list** of grounds for accent, danger and warning text. They are
   pressed and inset fills that carry `--color-text`; the runtime audit measures
   what is actually composited and is what would catch a counter-example.
6. **The tool-key dispatch check was made through the tool strip's live label and
   the unit tests**, not by pressing a tool letter in the browser, because the
   Edit tab opens in shaped-text mode where a bare letter is typed into the run by
   design.
7. **Native desktop verification remains out of scope**, as for every row of this
   phase: all of the above is the browser transport on Linux.

## Notes for the integrator

- `src/i18n/i18n.test.ts` fails until the translation pass; the key list above is
  what it needs from this task. `cmd.view.language.en` ships as "English
  Interface" and `cmd.view.language.zh-CN` as "中文界面 (Chinese Interface)"; the
  second is already Chinese and the first is deliberately English in both
  catalogues, since its job is to be legible to a reader who cannot read the
  current language.
- `commandHelp.test.ts` will fail for any command another task registers without
  a help entry, and names the ids. That is the guard working; adding the entry is
  one line in `shell/commandHelp.ts` plus one string under `quickHelp`.
- `cmd.view.glyph.verticalMode` (the CJK task's new command) is still untranslated
  in zh-CN and shows as English in the Chinese View menu; it is in the same
  translation delta.
- Build artifacts peaked at 26.40 GiB / 23,449 files while several agents built at
  once, which the checker reports as over the 20 GiB ceiling. This task ran exactly
  one Cargo command (`cargo build -p fc-devserver`, under the shared flock) and
  started no further build after the ceiling was crossed; another task's queued
  `cargo clean` then ran and the final check is **PASS at 6.54 GiB / 15,618 files,
  0.11 GiB temporary, 625.32 GiB free**. This task's own scratch directory and both
  of its servers are gone; the 0.11 GiB that remains belongs to the other areas.
- Changing the shape of a custom hook (`useModalChrome`) cannot be hot-replaced:
  Vite's Fast Refresh re-rendered the already-mounted `CloseDocumentDialog` with a
  different hook sequence and React tore the tree down. A full Vite restart
  (`--force`) and a reload clear it. It is an HMR hazard of the dev server, not a
  defect in the code — the same build mounts cleanly from a fresh load.

## Review round

The review returned ACCEPT WITH FIXES with five items. Each is fixed, each has a
test, and the two that could only be seen in a running browser were re-verified
there.

### 1. HIGH — the bubble's "Open help article" button was dead (row 107)

`installContextHelp` dismissed on `pointerdown` with no containment guard, so a
real click unmounted the bubble between `pointerdown` and `click`; it also
dismissed on every key, so a keyboard user could never reach the button. Four
changes:

- `pointerdown`, `focusin` and `keydown` all skip when the event is inside
  `[data-quick-help]`, so the bubble's own two controls work. Escape still
  dismisses from inside it.
- `pointerout` no longer closes immediately. Travelling from the control to the
  bubble produces a `pointerout` whose `relatedTarget` is `null` in Chrome, so an
  immediate dismissal always won the race. It now waits `QUICK_HELP_LEAVE_MS`
  (200 ms), which `pointerover` cancels the moment the pointer lands anywhere.
  The F1 hover target is still cleared at once, so that behaviour is unchanged.
- Tab from the control the bubble describes hands focus to "Open help article" —
  an interactive tooltip has to sit next to its trigger in the focus order, and
  the bubble is in a portal, so the order is made rather than inherited. A second
  Tab leaves and the focus change dismisses it.
- `pointerover` now closes a bubble when the pointer arrives at a _different_
  control, which is the dismissal `pointerout` used to do.

Tests: `quickHelp.test.tsx` replaces `fireEvent.click` with `userEvent.click`
(real pointer sequence, real timers) — **that test fails against the old code**;
plus new cases for a pointer press inside the bubble, for the pointer travelling
onto it, for Tab-then-Enter, and for Tab inside it not dismissing while Escape
still does. The leave test now advances past the grace period.

Browser re-verification on 8813/1513: a real `mouse down`/`mouse up` on the
button opened the article at `topic=export`; focusing **Toggle inspector**, then
Tab, put focus on "Open help article" with the bubble still open, and Enter
opened the article. Evidence: `quickhelp-keyboard-dark-en.png`.

### 2. MEDIUM — remaining British spellings

`infoBox.originPoint.4` ("Centre" → "Center", the one entry of that dropdown the
first pass missed) and `hints.ttAutohintNumbersHelp` ("behaviour" → "behavior").
`docs/user/appearance.md` and `docs/keyboard-shortcuts.md` are normalised too.

Test: new `app/src/i18n/spelling.test.ts` walks every leaf of `en.json` against a
list of British spellings and reports the key and the word. It is a new file, so
it does not touch the catalogue tests the translation pass owns.

### 3. MEDIUM — the bubble printed its sentence twice

`quickHelpTarget` now applies the same equality guard `openContextHelp` has: when
a control's description is its own name — an icon button labelled _and_ titled
"Toggle sidebar", a recent-file row described by the path it prints — the name is
kept and the topic summary becomes the description, which is the only sentence
that adds anything. Test: a new case asserting the name appears exactly once and
the summary is the topic's. Confirmed live: the bubble now reads "Toggle
inspector" once, with the font-overview summary under it.

### 4. LOW-MEDIUM — four holes in `a11y.test.tsx`

- An `id` only exempts a field when the file actually points an `htmlFor` at it.
- `tabIndex={-1}` is no longer a keyboard path, and only exempts a field when it
  is also `aria-hidden`. That flagged `views/text/ProofLibraryPanel.tsx`'s hidden
  file input, which now carries `aria-hidden="true"` (attribute-only patch).
- The icon-only rule covers `<Button>` and self-closing forms, not just
  lowercase `<button>`.
- The rendered walk covers **all thirteen** `DialogHost` kinds, asserting that
  each one actually renders (so the walk cannot go vacuous), carries `aria-modal`
  and an accessible name, and names every focusable control inside it.

Verified against planted regressions: an icon-only `<Button>`, an orphan `id`
field and a `tabIndex={-1}` clickable `<span>` were each reported with file and
line, and the file was restored. The first attempt at the tabIndex rule did _not_
catch the span — a backtracking regex — which is why it was checked rather than
assumed.

### 5. LOW — `--bm-*` had no classification test

`tokens.test.ts` now has the mirror of the `--cv-*` test: `PIXEL_MARKERS`
(`--bm-ink`, `--bm-accent`) owe 3:1 against `--bm-paper`, `PIXEL_RECESSIVE`
(`--bm-grid`, a hairline between pixels that would read as ink at 3:1) is
documented, and a new token in the family that is in neither list fails the
build. Verified by adding a `--bm-stray` token, which was reported.

### en.json keys changed in this round

- `infoBox.originPoint.4`: "Centre" → "Center".
- `hints.ttAutohintNumbersHelp`: "behaviour" → "behavior".

No key was added or removed in this round.

### Commands

```text
$ pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
    src/shell src/styles src/i18n/spelling.test.ts \
    src/views/text/ProofLibraryPanel.test.tsx \
    src/views/font/fontCommands.test.ts src/views/font/GlyphGrid.test.tsx
Test Files  36 passed (36)
Tests       302 passed (302)

$ pnpm --dir app exec eslint src/shell src/styles src/i18n \
    src/stores/keymapStore.ts src/views/text/ProofLibraryPanel.tsx
[exit 0, no output]

$ pnpm --dir app exec prettier --check src/shell src/styles src/i18n \
    src/views/text/ProofLibraryPanel.tsx
All matched files use Prettier code style!

$ pnpm --dir app check
No error in src/shell, src/styles, src/i18n or src/stores/keymapStore.ts.
```

Both servers and the browser session were stopped again and
`.scratch/p5f-m-help/` removed.
