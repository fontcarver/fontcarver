# Color text integration acceptance — 2026-09-30

Task: `superpowers/plans/tasks/p5-color-text-integration.md`.

Implemented optional color painters on canvas runs and passive glyphs; passive
paths can be null for glyphs whose content exists only in color layers. The
bridge retains shaped positions and omits the active editable glyph. Drawing
uses the supplied foreground ink in the existing translated y-up font frame,
while monochrome outlines and kerning reference ghosts retain their behavior.
The bounded text-cell loader now shares its loaded cell map with
`useGlyphColors` and includes color-only glyphs in preview data.

The proofing owner owns TextView's color-map wiring and the preview-controls
owner owns preview types and painting. Shared color loading, resource disposal,
backend formats, browser visual acceptance, and final broad frontend checks
remain with their assigned owners; this report does not claim those checks.

Regression tests first failed for omitted color-only entries and absent color
painting. After implementation, the exact focused command was:

```text
TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-color-text-integration pnpm --dir app exec vitest run src/views/text/drawTextRun.test.ts src/views/text/textCanvasBridge.test.ts src/views/text/useTextRunCells.test.tsx src/views/text/ghostRendering.test.ts src/views/text/previewPaint.test.ts

Test Files  5 passed (5)
     Tests  16 passed (16)
  Duration  5.13s
```

Targeted ESLint passed for `textCanvas.ts`, `textCanvasBridge.ts`,
`drawTextRun.ts`, `useTextRunCells.ts`, and their three test files. Owned files
were formatted with Prettier; `git diff --check` passed for those tracked files.
No FontForge implementation, new dependency, or user-visible string was added.
