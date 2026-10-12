# P5 metrics formula and synchronization UI acceptance

Date: 2026-09-30. Scope: `p5-metrics-ui.md`.

Implemented per-field layer overrides (`==`) and glyph keys (`=`), preserving
unrelated horizontal and vertical keys. Switching to a global formula clears
only that field's local override within one batch. Imported local formulas are
displayed with their local scope prefix. Clearing the last override restores
inheritance.

The inspector resolves the selected layer and invalidates previous answers when
the layer or effective keys change. It shows pending, in-sync, out-of-sync and
resolution-error states. Current-master and all-master synchronization use
`UpdateMetrics`, so the backend resolves again when applying. Current-master
updates are disabled for supplemental layers. Locked spacing disables formula,
numeric, and synchronization controls. Rejected formula drafts reset after the
model is refetched.

Validation (commands run in `app`, with absolute task TMPDIR):

```text
pnpm exec vitest run src/views/glyph/panels/MetricsPanel.test.tsx src/views/glyph/panels/metricsKeyCommands.test.ts src/views/glyph/panels/useResolvedMetrics.test.tsx src/views/glyph/panels/TextRow.test.tsx --maxWorkers=1
Test Files  4 passed (4)
     Tests  19 passed (19)

pnpm exec vitest run src/views/glyph/panels/EditInspector.test.tsx -t 'Metrics panel' --maxWorkers=1
Test Files  1 passed (1)
     Tests  2 passed | 26 skipped (28)
```

Scoped ESLint passed for the seven implementation/test files. Prettier was run
only on owned files; `git diff --check` passed. Earlier full inspector runs had
unrelated 5-second timeouts and a cascading reporter assertion while system load
exceeded 110; those runs are not recorded as passing. Root owns the final full
frontend suite and type check after regenerating API types.

Visual verification remains pending. An isolated agent-browser session failed
to launch Chrome before `DevToolsActivePort` was created, including the final
documented `--no-sandbox` retry. The first attempt reported zygote `Broken pipe`;
the final attempt produced no Chrome stderr. No screenshot or real-backend
browser result is claimed. The isolated Vite server, browser session, harness
and profile were stopped and removed after testing.

Cleanup removed 38 owned scratch files (2,080,023 bytes). Artifact checkpoint:
build 6.11 GiB / 13,180 files; temporary 0.23 GiB / 8,327 files; free 668.64 GiB;
PASS. The broader shared task owns the other temporary outputs.

New English messages are `panel.metrics.scopeHint`, `resolving`, `inSync`,
`resolveError`, `update`, `updateCurrent`, and `updateAll`. Chinese translation
was delegated to the independent localization owner. No generated API files,
backend files, dependencies, or FontForge source were edited by this task.
