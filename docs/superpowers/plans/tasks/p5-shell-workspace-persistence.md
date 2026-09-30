# P5 shell workspace persistence

## Scope

Save each document's latest tabs, selected tab, dock visibility and widths,
inspector section expansion, and text preview state in local browser storage.
Support named per-document presets with save, apply and delete actions. Migrate
an untitled document's session key when saved and restore a saved path when
reopened. Glyph tabs persist names rather than session-only glyph handles.

## Owned files and interfaces

- New `app/src/stores/workspaceStore.ts`, `workspaceSnapshot.ts`
  validation/adapters, and tests.
- New `app/src/dialogs/WorkspaceDialog.tsx`, CSS Module, and component tests.
- New `app/src/shell/workspaceLifecycle.ts` and tests (delegated nested agent).
- New `app/src/shell/WorkspaceDialogHost.tsx` and edit-handover regression tests.
- Narrow `workspace` namespace addition in `app/src/i18n/en.json`.
- Parent mounts dialog/lifecycle; existing UI/session stores remain read-only.

`captureWorkspace` and `restoreWorkspace` adapt document-specific snapshots to
the current shell with glyph-name maps. `workspaceDocumentKey` uses path when
available. The Zustand store exposes `saveCurrent`, `getCurrent`, `savePreset`,
`applyPreset`, `deletePreset`, and `migrateDocumentKey`.

## Acceptance

Write failing focused Vitest tests first. Cover independent document records,
reopening with new document/glyph handles, missing glyphs, tab selection,
per-document preset operations, Save As key migration, malformed or unsupported
storage, storage unavailability, snapshot isolation, and keyboard accessible
dialog actions. Coordinate broad app verification and interactive browser QA
with parent. Run artifact budgets at start and completion; no broad builds.

## Result

Implemented the bounded scope, including glyph-name remapping, session-only
unsaved-document keys, named preset limits, validated local storage, current
snapshot and preset isolation, fresh-storage merges for multiple document
windows, and edit settlement before applying a preset. Detached tab and panel
windows do not write partial document workspaces. Applying after navigation,
document close or dialog dismissal cancels safely.

Final focused Vitest runs: store **11 passed**, dialog **3 passed**, lifecycle
**13 passed**, dialog host **8 passed**; **35 tests total**. Store and dialog
tests began red before implementation; cross-window preservation regressions
also reproduced the prior loss before the merge fix. A loaded-machine dialog
run exceeded the default test timeout; replacing timer-heavy test interaction
setup with direct labelled-control events reduced the final three-test run to
385 ms. Focused ESLint and formatting checks passed. Parent owns broad app
gates, final browser verification, shell mounting and startup activation fix.
