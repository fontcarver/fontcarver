# P5 color rendering in text runs

Owner: color_rendering/text_integration, coordinated by
`p5-resume-coordination.md` and the color-rendering owner.

Scope: `textCanvas.ts`, `textCanvasBridge.ts`, `drawTextRun.ts`,
`useTextRunCells.ts`, and their focused tests. Reuse the color-rendering owner's
`ColorGlyphPaint` helper and `useGlyphColors` hook. The proofing owner wires the
returned color map into TextView; the preview-controls owner adapts preview
types and painting. No shared color helper or preview-painter edits here.

Interfaces: `TextCanvasRun.colors?: ReadonlyMap<GlyphId, ColorGlyphPaint>`,
`SurroundingGlyph.color?: ColorGlyphPaint`, nullable passive outline paths, and
`useTextRunCells` returning the color map plus optional color on preview glyphs.
Paint color glyphs in translated y-up font space, using the current ink for
foreground palette entries. Retain monochrome fallback and edit-mode omission.

Acceptance: focused Vitest coverage for color-only glyphs, positioned color
painting, monochrome fallback, bounded preview inclusion, and editable-glyph
omission. Run focused checks locally; the parent owns final broad frontend
checks and browser acceptance. Check artifact budgets at task boundaries.
