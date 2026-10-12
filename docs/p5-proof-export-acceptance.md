# P5 standalone proof export acceptance

Completed 2026-09-30. Bounded plan: `superpowers/plans/tasks/p5-proof-export.md`.

Implemented dependency-free `exportProofHtml(title, entries)` and
`exportProofPdf(title, entries)` in `app/src/views/text/proofExport.ts`.
Both preserve already-shaped glyph x/y positions and accept the absolute
M/L/Q/C/Z paths emitted by the kernel. HTML embeds escaped SVG outlines. PDF
converts quadratic curves to equivalent cubics, fills vector contours, paginates
A4 pages, and includes a valid byte-offset cross-reference table. Bounds include
curve control hulls, overhangs, advances, and vertical runs. Invalid path syntax,
nonfinite geometry, and unsupported size ranges are rejected before export.

No package dependencies, FontForge source, generated API files, or shared i18n
files were used or changed. UI integration and downloads belong to the parent
proofing task.

## Verification

Tests were written before implementation. The initial focused test run failed
because the module did not exist. Final commands, from `app/`:

```text
pnpm exec prettier --write src/views/text/proofExport.ts src/views/text/proofExport.test.ts
src/views/text/proofExport.ts
src/views/text/proofExport.test.ts (unchanged)

TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-proof-export pnpm test src/views/text/proofExport.test.ts
Test Files  1 passed (1)
     Tests  25 passed (25)

pnpm exec eslint src/views/text/proofExport.ts src/views/text/proofExport.test.ts
[exit 0, no diagnostics]
```

Tests cover HTML escaping, Unicode metadata and ActualText, positioned vectors,
quadratic conversion, vertical bounds, empty sheets, pagination, PDF object
byte offsets and stream lengths, visible fitted-size annotations, long Unicode
labels, malformed commands, illegal commas, and nonfinite coordinates. An
independent read-only reviewer identified the comma-parser gap; five regression
cases were added and the gap fixed. No other concrete review defects remained.

A temporary specimen builder extracted DejaVu Sans outlines from the existing
fixture using fontTools, then called the actual TypeScript exporters with Node.
The PDF skill marker was run once before PDF authoring. Poppler verification:

```text
pdfinfo .scratch/p5-proof-export/proof.pdf
Title: FontCarver outline proof / 校样
Pages: 2
Page size: 595.276 x 841.89 pts (A4)
File size: 53810 bytes
PDF version: 1.7
JavaScript: no

pdftoppm -scale-to 1100 -png .scratch/p5-proof-export/proof.pdf .scratch/p5-proof-export/proof
[exit 0; both rendered pages visually inspected]

pdftotext .scratch/p5-proof-export/proof.pdf .scratch/p5-proof-export/extracted.txt
[exit 0; original Unicode title and labels retained]
```

Both pages show correct counters, curves, baseline placement, vertical offsets,
wrapped long labels, page numbers, and no clipped text or outlines. Retained
review images:

- `screenshots/p5-proof-export-pdf-page-1.png`
- `screenshots/p5-proof-export-pdf-page-2.png`

Shared `pnpm check`, repository-wide lint/tests, and interactive download/UI
verification are intentionally delegated to the parent integration gate.

## Boundaries

PDF labels use built-in Helvetica without an embedded font. Non-ASCII label
characters display as unbroken U+XXXX tokens; their original Unicode remains in
ActualText and title metadata. Specimen outlines preserve all scripts without
installed font dependencies. Exceptionally long PDF titles show an ellipsis
with the full original retained in metadata and ActualText.

Oversized PDF runs fit the printable area and include an explicit numeric
`[requested -> actual]` point-size annotation. HTML has point-size dimensions
and responsive width fitting. The export describes a shaped snapshot; it does
not embed the original font or reshape text after export.

Own temporary specimen files and processes were cleaned after retaining the
report and final screenshots. Final artifact-check results are recorded in the
completion message; other tasks' caches and temporary files were left intact.
