# P5 live variation editing integration

## Scope

Rows 55–57: connect the existing interpolation sliders to a transient overlay
on the active glyph editing canvas; clear it on disable, failed requests and
owner changes; retain editable source geometry and refresh after document edits.
Improve bracket condition editing with mapped bounds, explicit validity and a
human-readable inclusive condition. Source correspondence/repair presentation is
delegated to `VariationCompatibility` with a separate bounded plan.

## Files and interfaces

- `variationPreviewStore.ts`, a document/glyph-scoped transient preview store.
- `drawVariationPreview.ts`, a plain canvas overlay renderer and focused tests.
- Narrow integration in `GlyphCanvas.tsx`, `renderGlyph.ts`,
  `GlyphVariationPanel.tsx`, `VariationPreview.tsx`.
- `BracketConditionFields.tsx` and narrow integration in `SourceLayerEditor.tsx`.
- Focused store/component tests and English `variation` i18n keys.

No model/API changes, generated files, shared audit/status files or commits.
Canvas edits are coordinated with outline, annotations and image workflow owners.

## Acceptance

Tests first for preview publication and cleanup, document/glyph isolation,
late response rejection, font-coordinate rendering and mapped/inverted/missing
condition bounds. Run focused Vitest tests and owned-file lint/format checks;
root performs final frontend/workspace checks. Use isolated agent-browser visual
verification if the shared devserver is available. Disk checks at start/end;
temporary files belong to `.scratch/p5-live-axes`.

## Acceptance report (2026-09-30)

Implemented a document/glyph-scoped transient preview store, main editing-canvas
outline/advance overlay, enable/disable cleanup, and request-identity rejection
of old previews. Source editing remains active; document events refresh the
interpolation after edits. Scrubbing issues no document mutations. The inspector
now uses the delegated source compatibility/point correspondence component.

Bracket conditions show mapped design-unit limits and an inclusive summary;
missing, nonfinite, out-of-range and reversed bounds disable Apply. Virtual
master presets fill all axes from the preset or mapped axis defaults, seed an
editable name and use the existing revision-checked Layer transaction.

Focused commands (all `TMPDIR` under this task; Vitest workers limited to one):

```
vitest run --maxWorkers=1 --minWorkers=1
  src/stores/variationPreviewStore.test.ts
  src/views/glyph/drawVariationPreview.test.ts
  src/views/glyph/panels/GlyphVariationPanel.test.tsx
  src/views/glyph/panels/bracketConditions.test.ts
  src/views/glyph/panels/SourceLayerEditor.test.tsx
Test Files 5 passed (5); Tests 8 passed (8)

vitest run --maxWorkers=1 --minWorkers=1
  src/views/glyph/panels/SourceLayerEditor.test.tsx
Test Files 1 passed (1); Tests 3 passed (3)
```

The last command adds the virtual-preset regression: nine unique owned tests
pass. The compatibility child reports 16 additional tests. Scoped Prettier and
ESLint on all twelve owned TS/TSX source/test files passed (exit 0). Root owns
generated API regeneration and final full checks; no generated files edited.

Browser attempt used an isolated headless agent-browser session and the existing
API 8897/UI 1431 pair. The initial shell rendered and a real, isolated two-master
acceptance document was created successfully. Concurrent Vite hot updates then
left the application blank; one navigation retry timed out. Full browser visual
acceptance is therefore **not passed**. The owned document and browser were
closed; no screenshot of a working feature is claimed. No server was started.
The renderer is a live interpolated overlay, not an editable synthetic instance.

Disk check before cleanup passed: build 6.11 GiB, temporary 0.30 GiB / 8,156
files (all concurrent tasks), free 668.66 GiB. Task scratch and short Chromium
TMPDIR `/tmp/fc-p5-axes` are removed at handoff; final disk check recorded in the
parent handoff. No dependencies or FontForge-derived code added.
