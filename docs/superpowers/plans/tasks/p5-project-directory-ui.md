# P5 directory project UI

## Scope and ownership

Add `.fcfp` directory-project support to the frontend session save checks and
native path dialogs. Own `nativeDialogs.ts` and its tests, narrow session-store
changes, an isolated project workflow test, and English strings. The shell
shortcut owner integrates the File menu/command palette entry; localization
independently updates Chinese. No Rust/API/generated file edits or dependencies.

## Behavior

Existing `.fcf` and `.fcf.gz` save choices remain first/default. A `.fcfp` save
path reaches the existing `session.save` API and subsequently saves directly.
Open Project Folder uses a directory picker in Tauri and the existing path
prompt in browser mode. Cancellation performs no open. Existing open-file
behavior stays files-only. Browser hints explain the project-directory suffix.

## Acceptance

Write failing tests for `.fcfp` direct save/save-and-close, path classification,
native directory selection, browser fallback, and the save filter. Run focused
Vitest and ESLint; parent runs final integrated frontend checks. Use an isolated
agent-browser session for the File menu and path-prompt visual checks. Record
artifact budgets before implementation and at completion; remove owned scratch
and browser/server processes after retaining acceptance evidence.
