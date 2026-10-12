# P5-15 smart-filter UI acceptance

Date: 2026-09-30

Implemented the named project-filter panel with selection, creation, editing,
deletion, retry, undo-event refresh, document isolation, and save ordering.
The editor switches between nested All/Any/None visual rules and a validated
custom expression. Values are quoted without losing regex escapes. Unicode
rules use scalar comparisons and accept `U+XXXX`. Search adds All/Name/Unicode/Note
scope, case matching, and regex controls. Filter chips and clear/reset behavior
cover the new options and exact glyph-name sets, including an empty set.

The sibling overview task integrated the panel in FontView and query controls
in GridToolbar. English keys are in `smartFilters` and `glyphSearch`; translation
is independently owned.

## Verification

Focused Vitest commands used an absolute task TMPDIR and `pnpm --dir app exec
vitest run`:

- `GlyphSearch.test.tsx`, `FilterChips.test.tsx`, and
  `stores/documentStore.test.ts`: **23/23 passed**; the final tooltip change
  reran `FilterChips.test.tsx`: **6/6 passed**.
- `SmartFiltersPanel.test.tsx`: **8/8 passed**, including save failure/retry,
  undo-event reload, pending-operation ordering and stale-document isolation.
- `SmartFilterEditor.test.tsx`: **3/3 passed**, covering builder/text round-trip,
  malformed DSL and preserving edits after a failed save.
- `smartFilterBuilder.test.ts`: final **4/4 passed**, including Unicode operator
  compatibility and nested/escaped expression serialization.

Scoped ESLint, Prettier and tracked-file whitespace checks passed. Some final
component runs used CLI `--testTimeout 15000` under concurrent machine load;
project test settings were unchanged. Resource check passed before cleanup:
build 6.12 GiB, scratch/reports 0.15 GiB, free space 668.73 GiB.

API generation, integrated TypeScript/full regression checks, independent
translations and browser visual acceptance remain central integration work;
they are not claimed here. This task started no browser or server. Its test
processes finished, and its owned scratch directory was removed.
