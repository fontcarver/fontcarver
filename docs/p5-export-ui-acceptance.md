# P5 export UI acceptance (2026-09-30)

Implemented persistent, versioned local export profiles (24 maximum), safe
profile application and removal, a named-instance/profile matrix (100 outputs
maximum), complete destination validation before writes, per-item results and
partial-failure continuation. Batch requests snapshot options and stop scheduling
new items after unmount or document changes. The design-space generation is
rechecked after settling pending edits. Preflight uses existing
`doc.find_problems`, shows actual findings and navigates to the affected glyph.

Processing controls normalize unsupported flags before requests. Geometry
preprocessing is disabled for variable outputs. Static CFF/OTF expose the
subroutinization option confirmed by the export backend owner. Bare CFF hides
TrueType instructions and sfnt table preservation. EPS requires a glyph name;
TTC currently exports one face through this dialog. Multi-document collections
remain available through API/CLI. CFF and Type42 offer static named instances
without a variable-font toggle. Source and bitmap formats send only their
supported options. Profiles and batch controls contain Enter so those workflows
do not accidentally trigger the single-export dialog action.

## Focused checks

All checks ran with one Vitest worker and a task-owned TMPDIR. Earlier transient
5-second test timeouts during heavily concurrent work were followed by a clean
retry under normal load, without raising timeouts. One final normalization check
coincided with removal of generated bindings during central regeneration; it
was rerun successfully after generation completed.

- `pnpm --dir app exec vitest run src/dialogs/ExportDialog.test.tsx src/dialogs/ExportMatrixPanel.test.tsx src/dialogs/exportOptionState.test.ts --maxWorkers=1`
  — 3 files, 35 tests passed.
- Delegated focused suites: `exportProfileStore.test.ts` 15,
  `ExportProfiles.test.tsx` 4, `ExportPreflight.test.tsx` 10,
  `exportMatrix.test.ts` 12 passed. Total 76 focused tests.
- Final CFF normalization recheck: `exportOptionState.test.ts` — 13 passed.
- Focused ESLint on every owned integration/delegated TypeScript file: exit 0.
- Prettier applied only to owned files.

Central integration owns broad application TypeScript, ESLint and regression
checks after API generation. These are not claimed here. Backend geometry and
codec behavior are validated by the backend owners, not by mocked UI tests.

## Browser verification

The agent-browser skill ran an isolated headless Chrome session against a
scratch Vite harness rendering the actual ExportDialog with mocked transport.
Verified profile save, preflight finding rendering, instance selection,
two successful batch reports, bare-CFF control availability, and glyph
navigation. The navigation state was `dialog: null` and active glyph tab
`glyph:1:7` for A. Screenshots were viewed and retained:

- `docs/screenshots/p5-export-preflight-matrix-light.png`
- `docs/screenshots/p5-export-matrix-results-dark.png`

No native desktop input or actual font export was used by the visual harness.
The Vite process and both owned browser sessions were stopped. The task's
scratch directory and external Chrome temporary directory were inspected and
removed. Shared build dependencies and other tasks' files were retained.
