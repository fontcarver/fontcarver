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
