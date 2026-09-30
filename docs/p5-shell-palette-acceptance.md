# P5 palette search acceptance

2026-09-30 · audit row 5 · owner: shell workspaces / palette routes.

Implemented category selection for commands, filters, local scripts and glyphs.
Commands, taxonomy filters, saved smart filters and discovered script names use
fuzzy search. Glyph names and Unicode queries use a debounced, metadata-only
`glyphs.list` request capped at 50 results; truncation is visible and late results
cannot replace a newer query. Filter routes replace the overview filter only
after pending edits settle; glyph routes use the existing tab transition barrier.
Local script selection loads source into the console without execution. Existing
disabled-command explanations, effective shortcut hints, keyboard navigation and
focus restoration remain available. New category controls stay within the modal
keyboard focus loop. All new display strings are in English `paletteSearch`;
translation is owned by the separate translation pass.

## Focused verification

Tests were added before helper/hook implementation and initially failed because
the new modules were absent. Final command:

```text
TMPDIR="$PWD/.scratch/p5-shell-palette/tmp" pnpm --dir app exec vitest run \
  src/shell/CommandPalette.test.tsx \
  src/shell/CommandPaletteRoutes.test.tsx \
  src/shell/paletteSearch.test.ts \
  src/shell/usePaletteGlyphSearch.test.ts

Test Files  4 passed (4)
     Tests  17 passed (17)
  Duration  18.07s
```

Focused ESLint on those tests, `CommandPaletteDialog.tsx`, `paletteSearch.ts`,
`usePaletteEntries.ts` and `usePaletteGlyphSearch.ts`: exit 0, no diagnostics.
Prettier formatted all changed palette sources and tests.

The parent owns broad app check/lint/tests, binding generation and real-browser
acceptance. At this report, the automation and saved smart-filter Rust types were
present but their generated TypeScript exports were still pending the shared
generation run. Component tests passed against the coordinated API contract.
No broad-build or browser pass is claimed here.

Artifact budget before work: build 5.23 GiB / 12,405 files, temporary 0.00 GiB /
1 file, free 670.47 GiB. Mid-verification checkpoint: build 5.88 GiB / 13,080
files, temporary 0.03 GiB / 2,621 files, free 669.55 GiB; PASS. Scratch cleanup
and a final checker pass completed before handoff. No task-owned server/browser
processes were started; all focused test/lint/format processes exited.
