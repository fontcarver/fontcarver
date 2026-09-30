# P5 directory project UI acceptance

Implemented `.fcfp` native-path recognition, open/save filters, the native
directory chooser, and browser path-prompt fallback. The shortcut owner wired
`file.openProject` into the File menu/command palette. `.fcf` and `.fcf.gz` remain
the first save extensions. Updated English open/save guidance; independent
localization was notified of `cmd.file.openProject`, `dialog.open.formats`, and
`dialog.saveAs.hint`.

## Verification

Initial new tests failed in 10 expected cases before implementation. Focused
session/project/native-dialog tests pass, including direct saves, save-before-
close, failed-save document retention, cancelled folder selection, selected
folder forwarding, browser fallback, and native extension filters.

```text
pnpm --dir app exec vitest run src/stores/sessionProjectPersistence.test.ts \
  src/shell/nativeDialogs.test.ts src/stores/sessionStore.test.ts
Test Files  3 passed (3)
Tests       34 passed (34)

pnpm --dir app exec eslint src/shell/nativeDialogs.ts \
  src/shell/nativeDialogs.test.ts src/stores/sessionStore.ts \
  src/stores/sessionProjectPersistence.test.ts
exit 0
```

Adding `builtinCommands.test.ts` produced 51 passes and one catalogue failure for
10 unrelated concurrently introduced path/layer/window/workspace keys. The new
`cmd.file.openProject` key resolved. Parent owns final integrated checks.

## Acceptance limits and cleanup

Agent-browser was attempted on an isolated browser and ports 1464/8864. Its
initial Chrome startup failure came from the task TMPDIR exceeding the Unix
socket path length; a short task-owned `/tmp/fontcarver-p5-project-ui` resolved
startup. Concurrent source updates then caused repeated Vite reloads before the
root mounted. No rendered UI acceptance or native platform execution is claimed.
No screenshot is retained because none showed the UI. Backend project writes
and reads are accepted separately by the project-persistence owner.

Owned browser sessions and Vite/devserver processes were stopped. Owned scratch
and short temporary directories were removed. Resource checks passed; initial
build use was 5.69 GiB, pre-cleanup 6.07 GiB (shared concurrent builds), and owned
scratch was 11 MiB. No shared caches, user fonts, or source files were removed.
