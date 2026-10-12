# Visual lookup shaping preview

Scope: provide an explicit preview of applied feature shaping with one selected OpenType tag enabled and disabled. The preview shows the resulting glyph names, clusters, advances, offsets, and total advance difference. Pending visual/source drafts are identified as unapplied. No shaping call occurs on mount.

Files: `app/src/views/features/VisualLookupPreview.tsx`, its CSS module and tests; narrowly append `visualLookupPreview` English messages. Parent task owns integration into the feature editor. No generated APIs or Chinese translations are edited.

Public interface: `VisualLookupPreview({ doc: number, master: string | null, generation: number, tag: string, hasDraft: boolean })`.

Acceptance: transport tests verify actual paired `text.shape` requests with explicit feature toggles, metric/glyph presentation, draft notice, invalid tag/input guards, and rejection of obsolete responses on input/document/master/compile generation changes. Run app type/lint checks and focused Vitest coverage, coordinating broad suite and browser acceptance with parent. Resource checks run at start and completion.

## Acceptance outcome

Implemented paired applied-source shaping, glyph and metric comparison, capped explicit text entry, draft notice, outdated-font notice, and request identity cancellation. Preview requests use API automatic script/language/direction detection (`null`). Parent owns editor integration and integrated browser/full-suite acceptance.

- `pnpm --dir app exec vitest run src/views/features/VisualLookupPreview.test.tsx`: `Test Files 1 passed (1); Tests 8 passed (8)` (final duration 4.78s).
- `pnpm --dir app exec eslint src/views/features/VisualLookupPreview.tsx src/views/features/VisualLookupPreview.test.tsx`: exit 0.
- `pnpm --dir app check`: exit 2, blocked by current workspace generated API drift and unrelated application errors (missing automation API, export options/commands, SVG element typing, proof sheet master typing); reported to parent rather than widening scope.
- `python3 scripts/check_artifacts.py`: PASS; build 6.10 GiB / 13,178 files; temporary 0.16 GiB / 5,931 files; free 668.70 GiB. Task scratch removed; no task process remains.

No FontForge code, new dependencies, or generated API changes.
