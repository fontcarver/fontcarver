# Export preflight acceptance — 2026-09-30

`ExportPreflight({ doc, busy, onNavigate })` is ready for parent integration in
ExportDialog. Its explicit action calls `doc.find_problems` after pending edits
settle; mounting the panel makes no transport request. It displays actual
severity counts, omitted-result totals, errors including omitted errors, exact
diagnostic details, locations, and stale-report notices. Glyph shortcuts use
the existing tab transition helper and verify the document, generation, panel
lifetime, and dialog before opening a glyph and calling `onNavigate`.

Focused validation:

```text
TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-export-preflight pnpm exec vitest run src/dialogs/ExportPreflight.test.tsx
Test Files  1 passed (1)
     Tests  10 passed (10)

pnpm exec eslint src/dialogs/ExportPreflight.tsx src/dialogs/ExportPreflight.test.tsx
Exit 0; no diagnostics.
```

Tests cover explicit-only scanning, counts/truncation, exact feature errors,
draft settling, keyboard navigation without document mutation requests,
stale-result refresh, delayed retargeted responses, unmount before a scan,
navigation cancellation on edits/unmount, retry, busy state, and closed docs.
Prettier ran only on the two owned TypeScript files. The first test run failed
as expected because the component did not yet exist.

The parent owns ExportDialog integration, full frontend checks, and browser
visual acceptance. No generated API or translation files were edited; English
keys are under `exportPreflight`. No new dependencies or FontForge sources were
used. Task-owned temporary Vitest data is removed after the final focused run.
