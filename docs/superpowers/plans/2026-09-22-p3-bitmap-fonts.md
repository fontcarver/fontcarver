# P3 — Bitmap fonts

Status: implementation and listed automated/browser checks complete, 2026-09-22.
Native desktop acceptance and full bitmap-font-viewer workflow parity are not
established by these checks. User authorized all P3 work on 2026-09-22.
Supported variants, final checks and the existing layout-test baseline failure
are recorded in [P3 acceptance](../../p3-acceptance.md).

## Contract

Complete the binding P3 roadmap: editable bitmap strikes, BDF/PCF/FNT/FON/PSF,
OTB and embedded EBDT/CBDT/sbix, rasterization, tracing and waterfall preview.
Use the existing document/undo/API/export paths. Preserve outlines on bitmap
edits and preserve unrelated binary tables. Reject malformed or unrepresentable
inputs explicitly. No silent encoding, metric, color or strike loss.

Pixels use top-to-bottom, MSB-first, byte-aligned rows. Origins are the lower-left
corner in y-up pixel coordinates. Depths 1/2/4/8 are coverage; 32 is RGBA.
Strikes are identified by (ppem, depth), never by their position. Resource limits
are checked before allocation. Bitmap edits have separate undo snapshots and
invalidation, without marking outline hinting as touched.

## Steps

- [x] 01 Model validation, pixel operations, reference resolution and undo.
- [x] 02 BDF read/write, API/CLI opening and export, persistence regressions.
- [x] 03 Bitmap editor, strike management, tools, metrics and clipboard.
- [x] 04 PSF1/2, PCF and Windows FNT/FON with interoperability tests.
- [x] 05 Embedded EBDT/EBLC, CBDT/CBLC, sbix and OTB output.
- [x] 06 Outline rasterization, bitmap tracing, regeneration and waterfall.
- [x] 07 Chinese localization, browser workflows, CJK corpus and acceptance.

## Verification

Each step adds meaningful malformed-input, semantic round-trip or interaction
tests. Final gates: workspace tests/Clippy/format, API generation, frontend
typecheck/lint/tests/build, isolated browser workflows and visual inspection.
Use independent tools (FreeType/fontTools/X11 tools when available) to validate
binary formats. Synthetic fixtures are project-authored; external corpora stay
read-only. Record supported format variants and actual evidence in
`docs/p3-acceptance.md` before marking this milestone complete.

Existing performance investigations, native desktop input automation, release
packaging and P4/P5 remain outside P3. Do not execute FontForge.
