# SVG color image serialization

Scope: `app/src/canvas/svgGlyphImage.ts` and focused serializer tests. The color
rendering owner integrates image loading and disposal; no other canvas or SVG
editor files are owned by this subtask.

Interface: `serializeSvgGlyph(source, id, unitsPerEm, bounds?)` returns an SVG
image source or `null`. Bounds use SVG's y-down coordinates relative to the font
baseline; `defaultSvgGlyphBounds(unitsPerEm)` supplies the existing preview's
frame. The source stays detached from the application DOM and is only suitable
for loading in browser image mode.

Acceptance: first add failing tests for malformed XML, source byte budget,
missing targets, invalid bounds, preserved nested transforms/presentation,
shared local definitions, and source-root selection. Implement and run only the
focused tests and file-specific lint/format checks. The parent owns broad checks.
Check artifact budgets at task start and completion, then record actual results.

## Acceptance — 2026-09-30

Implemented the serializer and 12 tests. Selected glyphs retain ancestor
transforms, inherited presentation, shared definitions, and local outline
references; paths under `defs` become renderable without painting sibling
glyphs. Caller-supplied CPAL colors and foreground become inherited CSS values.
Image dimensions are capped at 512 pixels while the viewBox stays in font units.
The helper never inserts source nodes into the application DOM.

The first test run failed because the implementation module did not exist.
Final focused verification:

```text
pnpm --dir app exec vitest run src/canvas/svgGlyphImage.test.ts
Test Files  1 passed (1)
     Tests  12 passed (12)

pnpm --dir app exec eslint src/canvas/svgGlyphImage.ts src/canvas/svgGlyphImage.test.ts
exit 0

pnpm --dir app exec prettier --check src/canvas/svgGlyphImage.ts src/canvas/svgGlyphImage.test.ts
All matched files use Prettier code style!
```

The color-rendering owner handles image lifecycle, canvas integration, broad
frontend checks, and browser acceptance. The existing authoring preview was
left unchanged. No dependencies, generated files, or FontForge code were used.
The task's empty scratch directory was removed after focused checks.
