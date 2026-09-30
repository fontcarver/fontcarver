# P5 color rendering integration

Owner: color_rendering, with SVG serialization and text integration subagents.

## Scope and interfaces

Add bounded, read-only rendering of editable COLRv0, static COLRv1 and SVG glyphs
in inventory thumbnails, the text canvas and proofing preview strip. Reuse
`color.state`, `color.preview`, `color.graph_preview`, `svg.state`; never decode
imported raw data by mutating the document. Keep backend graph/source work limits
and isolated SVG image mode. Prefer COLRv1, then COLRv0, then SVG when present;
unsupported/failed color rendering retains the outline fallback.

`canvas/colorGlyph.ts` exports `ColorGlyphPaint.paint(context, foreground)` in
y-up font coordinates. `views/color/useGlyphColor.ts` shares bounded visible-glyph
loads by document generation and master. Shared requests dispose FontFace/blob
resources when released. Text and cell renderers call the same painter, preserving
shaped positions and palette alpha/currentColor.

No model/codec ownership changes, generated API edits, translations, shared status
edits or commits. Nondefault-master and variable color support must match the API
actually exposed and be reported accurately. Proof exports remain outside scope.

## Acceptance

Write focused tests first for alpha/currentColor, request deduplication,
generation/master invalidation, disposal, bounded concurrency and SVG source
isolation. Test color-only glyphs without monochrome outlines in grid/text paths.
Run focused Vitest/ESLint plus TypeScript check when coordinated; parent owns broad
checks. Use agent-browser for isolated visual verification of grid/text color
rendering. Record actual commands and static/variable limits in acceptance report.
Run artifacts checker at boundaries and remove owned superseded scratch outputs.
