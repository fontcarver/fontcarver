# Scoped information acceptance — 2026-09-30

The Font Info tab now opens a staged editor for existing font identity, UPM,
version, authorship and legal fields. See `p5-font-metadata-acceptance.md` for
the editing, conflict and failure cases. Masters have a direct design-space
editor entry. Optional master metrics show the current effective compiler
defaults and their source; enabling an override copies that effective value,
explicit zero remains an override, and clearing it resumes calculation from
current master metrics or font UPM. Both editor and comparison table show the
source. Export instances show the font family fallback and instance-name style
fallback, distinguish explicit export overrides, and can restore the font family.

## Verification

Tests were written before implementation. The initial focused run failed on the
missing metric resolver, zero placeholders instead of UPM, and absent naming
inheritance indicators. After implementation:

```
pnpm --dir app exec vitest run --maxWorkers=1 --no-file-parallelism \
  src/views/fontinfo/FontInfoPanel.test.tsx \
  src/views/fontinfo/ScopedEditors.test.tsx \
  src/views/fontinfo/metricInheritance.test.ts
Test Files  3 passed (3)
     Tests  16 passed (16)
  Duration  11.41s
```

The earlier FontInfoPanel/designSpaceSession regression run passed 14 tests;
the new component/default-policy run passed five. A later parallel run under
heavy shared load took 114 seconds and had four timeout/async-timing failures;
the serial run above passed without increasing timeouts. Scoped ESLint,
Prettier and `git diff --check` passed. Final workspace TypeScript/regression
checks remain the coordinating agent's responsibility.

An isolated agent-browser session could not launch Chrome: it exited before
writing DevToolsActivePort, including one retry with `--no-sandbox`. The local
doctor reported 13 passes, one unrelated stale-state warning and no failures.
No screenshots or browser accessibility pass are claimed. The owned browser,
Vite and development-server processes were stopped and their scratch profile
removed. No Rust API or generated types were changed by this task.

## Audit boundaries

- Row 52 remains partial: the implemented inheritance is font family defaults
  to export instances, plus master metric defaults/overrides. The model has no
  multi-document family container or arbitrary Family → Font → Master → Export
  metadata hierarchy. This task does not claim one.
- Row 53 gains editable UPM and honest master metric sources. Existing
  PostScript editors provide per-master zones/stems. Grid subdivision and
  preference evidence belong to the separate snapping-preferences task.
- Backend command undo and `.fcf` paths are existing functionality; this task
  did not rerun Rust tests or claim new binary-export coverage. Imported
  localized name records remain preserved by the metadata editor.
