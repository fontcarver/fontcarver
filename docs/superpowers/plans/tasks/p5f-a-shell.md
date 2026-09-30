# p5f-a: Shell, windows, workspaces and project file

Area id: `a-shell`. Ports: API 8801, UI 1501. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                                        | State at start                                                                                                          |
| --- | ------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| 1   | One window per document; tab strip with pinned Overview tab (Mod+Alt+1) + Edit tabs (Mod+Alt+2–9, Ctrl+Tab cycles) | Tabs and cycling exist; per-document windows, pinned Overview and the ordinal bindings need acceptance.                 |
| 2   | Tear-off tabs and panels into separate OS windows                                                                  | `DetachedPanel`, `windowLifecycle`, `DetachedWindowBar` exist; see `docs/p5-shell-windows-acceptance.md`.               |
| 3   | Right Inspector dock as a contextual scrolling section stack; left context sidebar                                 | Reusable evidence (P1); attach it and verify against the current shell.                                                 |
| 4   | Collapsible bottom preview strip with draggable divider                                                            | Strip exists; the draggable divider is unverified.                                                                      |
| 5   | Command palette (Mod+K) over commands, filters, scripts, glyph names                                               | `paletteSearch`, `usePaletteEntries`, `usePaletteGlyphSearch`; see `docs/p5-shell-palette-acceptance.md`.               |
| 6   | Saved workspaces, restored per document                                                                            | `workspaceStore`, `WorkspaceDialog`, `workspaceLifecycle`; see `docs/p5-shell-acceptance.md`.                           |
| 7   | Text-first project format (per-glyph files), git-friendly; crash-recovery snapshots in a sidecar                   | `.fcfp` directory projects exist (`crates/fc-doc/src/fcf/schema.md` "Directory projects"); recovery sidecar to verify.  |
| 8   | Document-level undo stack with per-entry scope labels; one entry per gesture                                       | History and labels exist; the user-visible per-entry scope label surface needs acceptance.                              |
| 9   | Cross-platform parity: macOS, Windows, Linux, identical keymaps modulo Cmd/Ctrl                                    | Key abstraction exists; verify the abstraction and platform detection; native macOS/Windows runs stay pending (task n). |

Existing records to read: `docs/p5-shell-acceptance.md`,
`docs/p5-shell-windows-acceptance.md`, `docs/p5-shell-palette-acceptance.md`,
`docs/acceptance/p5-shell/`, `docs/superpowers/plans/tasks/p5-shell-*.md`,
`docs/p1-acceptance.md` (shell sections), `docs/design/editor-ui.md`.

## Scope notes

- Row 1: in the Tauri shell a document opens in its own OS window; in the
  browser development transport, emulate with `window.open` or document the
  single-window fallback honestly, but the Tauri path must be implemented
  (`src-tauri/src`, capabilities) and at least `cargo check -p fontcarver`
  (the `src-tauri` package) must pass. Overview tab pinned first and not
  closable; Mod+Alt+1..9 and Ctrl+Tab/Ctrl+Shift+Tab wired through the keymap.
- Row 2: tearing a tab or a panel off produces a separate window that stays in
  sync with the document; closing it returns the content. Cover keyboard access.
- Row 4: pointer drag on the divider resizes the preview strip; the height is
  persisted with the workspace; collapse/expand has a command and a shortcut.
- Row 5: the palette searches commands, saved/smart filters, scripts and glyph
  names in scoped sections; Mod+K is the binding; results are keyboard
  navigable; selecting a glyph opens its Edit tab.
- Row 6: named workspaces (panel layout, sidebar state, preview height,
  detached windows) saved in the project and restored per document on reopen.
- Row 7: verify that `.fcfp` output is deterministic (stable key order, stable
  glyph file names, trailing newline) so that a re-save without changes
  produces no git diff; verify that crash-recovery snapshots are written to a
  sidecar next to the project (or the documented recovery location with a
  pointer) and that reopening after a simulated crash offers recovery; document
  the layout in `docs/user/` if missing.
- Row 8: a History panel or Edit menu surface lists entries with scope labels
  (glyph name / layer / font-level), one entry per gesture (drag, typed field
  commit, dialog apply); undo/redo through it works.
- Row 9: a single keymap definition with a Mod abstraction; Linux evidence now;
  record macOS/Windows as pending native execution.

## Owned files

`app/src/shell/{AppShell*,TabBar*,TabContent.tsx,tabActions.ts,windowActions.ts,windowLifecycle.ts,DetachedPanel.tsx,DetachedWindowBar.tsx,WorkspaceDialogHost.tsx,workspaceLifecycle.ts,workspaceTabCommands.ts,Inspector.tsx,Sidebar.tsx,CommandPalette*,paletteSearch.ts,usePaletteEntries.ts,usePaletteGlyphSearch.ts,fuzzy.ts,MenuBar*,MenuItem.tsx,StatusBar*,RecoveredBanner.tsx,CloseDocumentDialog.tsx,EmptyState*,panels.module.css,useCommandRegistry.ts}`
and their tests; `app/src/dialogs/WorkspaceDialog*`, `app/src/dialogs/RecoveryDialog.tsx`,
`app/src/dialogs/recovery.ts`; `app/src/stores/{uiStore,workspaceStore,workspaceSnapshot,sessionStore,sessionProjectPersistence,sessionTabBarriers}*`;
`src-tauri/**`; `crates/fc-doc/src/{fcf/**,autosave.rs,autosave_tests.rs,undo/**,undo.rs,session.rs}`;
`crates/fc-api/src/handlers/{session.rs,doc.rs,prefs.rs}`, `crates/fc-api/src/types/session.rs`;
`app/e2e/app.spec.ts`, `app/e2e/panels.spec.ts`; `docs/p5f-a-shell-acceptance.md`; user docs for the shell.

Shared files (narrow patches only): `app/src/shell/builtinCommands.ts`,
`app/src/shell/commands.ts`, `app/src/shell/shortcuts.ts`,
`app/src/dialogs/DialogHost.tsx`, `app/src/i18n/en.json`,
`crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`,
`crates/fc-doc/src/lib.rs`.

## Gap analysis

Read on 2026-09-30 against `HEAD 449e753`: `AppShell.tsx`, `TabBar.tsx`,
`tabActions.ts`, `workspaceTabCommands.ts`, `shellCommands.ts`,
`builtinCommands.ts`, `windowActions.ts`, `windowLifecycle.ts`,
`transport/windows*.ts`, `src-tauri/src/{lib,windows}.rs`, `uiStore.ts`,
`workspaceStore.ts`, `workspaceSnapshot.ts`, `workspaceLifecycle.ts`,
`WorkspaceDialog*.tsx`, `Inspector.tsx`, `Sidebar.tsx`, `CommandPalette*`,
`paletteSearch.ts`, `usePaletteEntries.ts`, `views/text/PreviewStrip.tsx`,
`crates/fc-doc/src/{undo.rs,autosave.rs,fcf/**}`, `crates/fc-api/src/handlers/{session,doc,prefs}.rs`.

| Row | Works now                                                                                                                                                                                                                                                                                                                       | Missing                                                                                                                                                                                                                                                             | Decision                                                                                                                                                                                                                                                                                                                                                                                                             |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Desktop routing of a second document to its own window (`windowLifecycle.reconcile`), Tauri `open_workspace_window` with label reuse/focus, `Mod+Alt+1..9`, `Ctrl+Tab/Ctrl+Shift+Tab`, no close button on the Overview.                                                                                                         | Overview is only first by open accident, not by invariant; `Mod+Alt+2..9` skipped Font Info/Features/Bitmap tabs (glyph tabs only); `Mod+Shift+W` (Close Tab) would close the whole document from the pinned Overview; no pinned affordance or `aria-keyshortcuts`. | Implement: pin the Overview first in `uiStore.openTab`; ordinals address every non-Overview tab of the active document in strip order; disable Close Tab on the Overview with a reason; pinned marker + `aria-keyshortcuts` in `TabBar`. Verify `cargo check -p fontcarver`.                                                                                                                                         |
| 2   | Tab tear-off button + drag-out, sidebar/inspector detach commands and panel buttons, preview detach button, rejoin bar, browser popup + Tauri window, close returns the content.                                                                                                                                                | No command (so no palette/keyboard route) for detaching the preview; detached windows are not recorded anywhere, so nothing can restore them.                                                                                                                       | Implement `window.detachPreview`; record detached targets in `uiStore` so row 6 can restore them and the Window menu can report them.                                                                                                                                                                                                                                                                                |
| 3   | `Inspector` renders a scrolling `panelStack` whose content follows the active tab, `EditInspector` sections remember open/closed through `uiStore.sections`; `Sidebar` is contextual per tab kind.                                                                                                                              | Only acceptance evidence is missing.                                                                                                                                                                                                                                | Verify in the browser and with focused tests; no code change expected.                                                                                                                                                                                                                                                                                                                                               |
| 4   | `PreviewStrip` divider: pointer capture drag, Arrow/Home/End keys, `role="separator"` with `aria-valuemin/max/now`, height bounded 120–600 and persisted per tab in `textState.previewHeight`, captured by the workspace snapshot; collapse chevron.                                                                            | Collapse/expand has no command and no shortcut; the existing `view.textPreview` command has no shortcut and toggles strip visibility, not collapse.                                                                                                                 | Add a shortcut to `view.textPreview` and a separate collapse/expand command with a shortcut, acting on the active Edit tab. `PreviewStrip.tsx` belongs to task h, so the commands write `textState` through `uiStore.updateTab`.                                                                                                                                                                                     |
| 5   | `Mod+K` (`palette.quickOpen`), scoped search over commands, taxonomy filters, saved smart filters, local scripts and debounced glyph metadata search, keyboard navigation, focus restore, disabled-reason rows, glyph selection opens the Edit tab.                                                                             | Results are one flat list: the row asks for scoped **sections**.                                                                                                                                                                                                    | Group the `all` scope into labelled sections (`role="group"`) keeping one flat keyboard traversal order.                                                                                                                                                                                                                                                                                                             |
| 6   | Per-document automatic layout + named presets (tabs, panel visibility/sizes, inspector sections, preview visibility, per-tab preview height) in validated browser storage keyed by document path, glyph-name remapping, Save As key migration, cross-window merge.                                                              | Presets live only in browser storage, so they do not travel with the document (the scope note asks for the project); detached windows are not part of the snapshot.                                                                                                 | Implement a kernel-side project store: `session.workspaces_get/_set` writing a deterministic `workspaces.json` sidecar — inside the directory for `.fcfp`, `<path>.workspaces.json` for `.fcf`/`.fcf.gz` — synced into the existing store. Non-native documents (an opened `.ttf`) keep browser-local records only; recorded as an explicit boundary. Add detached windows to the snapshot and reopen them on apply. |
| 7   | `.fcfp` manifest + content-addressed per-glyph files, atomic manifest commit, advisory lock, pruning limited to the previous manifest, symlink/duplicate/mismatch rejection, manual text edits accepted, recovery snapshots in the configured recovery directory with a `document_path` pointer and project-manifest freshness. | No test proves a no-change re-save is byte-identical (the git-friendliness claim); no test proves a crashed `.fcfp` is offered for recovery end to end; `docs/user/` has no project/recovery page.                                                                  | Add determinism and recovery integration tests, and a user page. Recovery stays in the documented recovery location with the pointer — the scope note allows it explicitly — rather than moving snapshots next to the source; recorded as the chosen option, not a limit.                                                                                                                                            |
| 8   | One undo entry per gesture with coalescing, `undo_label`/`redo_label` in the summary, Edit-menu Undo/Redo carrying the next label, status-bar flash.                                                                                                                                                                            | No list of entries anywhere: no API, no scope labels, no panel, no jump.                                                                                                                                                                                            | Implement `UndoStack::entries()` with a scope computed by diffing an entry's inverse/forward snapshots (glyph names, layer names, font-level), a one-line `Document::history()` accessor, `doc.history` and `doc.history_go` handlers, and a History section in the Inspector with a keyboard-navigable list plus an Edit-menu command.                                                                              |
| 9   | One `Mod` notation parsed in `shortcutSyntax.ts`, `isMacPlatform()` from `navigator`, one command table used by menu, palette, dispatcher and the shortcut editor.                                                                                                                                                              | No test asserts that the same table resolves identically on the three platforms modulo Cmd/Ctrl; macOS/Windows runtime remains unexecuted.                                                                                                                          | Add a platform-parity test over the real command table (the keymap modules belong to task m, so the test only reads them). macOS/Windows native execution stays pending task n.                                                                                                                                                                                                                                      |

Shared files I will patch narrowly: `app/src/shell/{builtinCommands,shellCommands}.ts`
(command registration only), `app/src/i18n/en.json` (new `history`,
`shellWindow`, `workspace`, `cmd.*` keys), `crates/fc-api/src/handlers/mod.rs`
and `crates/fc-api/src/types/mod.rs` (new route/type exports),
`crates/fc-doc/src/document.rs` (one `history()` accessor),
`crates/fc-doc/src/fcf/schema.md` (project sidecar).
