# P5 bitmap PNG sprite sheets

Implement bounded PNG sprite-sheet import/export in the bitmap editor. Own
`app/src/views/bitmap/BitmapSheet.tsx`, `bitmapSheet.ts`, their tests, a CSS
module, and a plain browser canvas adapter. Parent mounts the component and
provides the atomic `BitmapCommand::SetGlyphs` command. English strings use
`bitmapSheet`; independent localization owns translations.

Public component: `<BitmapSheet doc={doc} strike={strike} />`, where strike is
`BitmapStrikeInfo`. Names are explicit and row-major, at most 256 unique names.
Fixed cells align bitmap pixels at their bottom left. Existing bitmap dimensions,
origins, horizontal/vertical advances, and vertical origin survive import; ink
outside existing bounds is rejected. Missing images use complete cell dimensions
and default strike baseline metrics. Import materializes referenced images.
All selected glyphs update in one undoable command without replacing other glyphs.

PNG input is limited to 16 MiB, dimensions to 8192, and decoded pixels to 4 Mi.
Read the PNG signature/IHDR before browser decoding; reject invalid mapping,
dimensions, missing glyph names, and generation changes. Monochrome/grayscale
coverage uses PNG alpha; RGBA preserves channels. Export refuses clipped glyphs.

Acceptance: test independent cell coordinates, RGBA/alpha/depth, exact existing
metric preservation, missing images, duplicate names, truncation and bounds.
Test import atomic request and stale-generation rejection. Run focused Vitest,
TypeScript and ESLint; parent owns broad integration. Verify via isolated
agent-browser when the shared devserver is ready. Check artifact budget at start
and completion, and remove owned temporary artifacts.
