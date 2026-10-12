# P3 bitmap fonts acceptance

P3 implementation and the automated/browser checks below are complete within the
documented format boundaries. This is not a claim of exhaustive format coverage,
native desktop acceptance or verified parity with every bitmap-font-viewer
workflow. Editable strikes use the existing document, history, save, import/export
and browser UI. Verification was performed on 2026-09-22. Native desktop input
automation and release packaging have not been verified.

## Editing and document behavior

- The Bitmap menu opens a dedicated editor with a paged glyph list, Unicode
  search, strike creation/removal, glyph and strike metrics, and BDF properties.
- Tools include pencil, eraser, line, rectangle, ellipse, fill, floating rectangular
  selection, copy/cut/paste, flip, 90-degree rotation, pixel shifts, crop, inversion,
  and nearest-neighbor or interpolated grayscale/RGBA scaling. Canvas transforms
  and paste clip at its bounds; resizing preserves baseline coordinates. The
  internal clipboard requires matching depths.
- Coverage depths are 1/2/4/8; color is straight RGBA8 (32 bpp). Pixels are packed
  MSB first in byte-aligned, top-to-bottom rows, with zero padding bits. Bitmap
  origins use y-up coordinates at the lower-left corner.
- Pixel and metadata edits participate in document undo/redo and `.fcf` save,
  autosave and reopen. Focused numeric drafts are flushed before saving. Failed
  writes retain local pixels and prevent navigation until retried or explicitly
  discarded. Generation checks reject stale writes.
- Editing pixels does not mark source outlines as edited or discard their
  eligible TrueType instructions. Glyph renaming updates bitmap references;
  duplication copies images; deleting a referenced glyph resolves surviving
  bitmap components first.
- Decoded embedded tables belong to the editable model. Export replaces those
  tables, including after deletion of the last strike, so old images cannot
  reappear from preserved binary data. Unrelated tables remain preserved.

## Format boundaries

| Format | Implemented behavior and explicit limits |
| --- | --- |
| BDF | Read 2.1/2.2/2.3; write 2.1 or 2.2 with vertical metrics. Coverage 1/2/4/8, bearings, advances, vertical origin, typed properties and glyph attributes. Unicode ISO10646 and Latin-1 mappings; other charset codes stay tagged legacy codes. Multiple Unicode aliases per glyph cannot be represented by this writer. |
| PCF | Monochrome, names, properties, scalable widths, bearings, BMP encoding; compressed and uncompressed metrics, both endian/bit orders and scan-unit/padding combinations. One encoding per glyph; vertical metrics cannot be exported. |
| PSF | PSF1 and PSF2 read/write, fixed monochrome cells and Unicode aliases. PSF1 requires 256 or 512 glyphs of width 8; PSF2 supports supplementary Unicode scalars. Unicode sequences are rejected. PSF carries no baseline/bearing metadata; export warns about that format limitation and rejects clipping. |
| Windows FNT/FON | Read raster FNT v2/v3; write v3. Read/write NE FON collections containing same-family strikes. Monochrome, byte-sized character encodings, CP1252 mappings or preserved legacy codes. Vector/color FNT and unrepresentable encodings are rejected. |
| EBDT/EBLC | Editable coverage strikes; read the fontations-supported index formats 1–5, mask data and composites. Write byte-aligned image formats 1/6 using the compiler's actual glyph order. Independent horizontal and paired vertical metrics are supported. |
| CBDT/CBLC | Editable PNG/RGBA color strikes; write PNG formats 17/18 with horizontal or big metrics. Empty images use raw bitmap records. Raw premultiplied BGRA inputs are converted to straight RGBA. |
| sbix | Read PNG and bounded `dupe` chains; write PNG. Other graphic types are rejected. Advances must equal scaled outline advances; use CBDT for independent bitmap metrics. Vertical overrides are rejected. |
| OTB | Write bitmap-only OpenType with EBDT and empty outlines. Export reports the loss of outline/layout data; the source document retains it. Reopens through the normal application and CLI paths. |
| TTF/OTF/WOFF/WOFF2 | Normal compilation includes editable embedded strikes before container wrapping. Coverage uses EBDT, RGBA uses CBDT or the imported sbix flavor. Dedicated CBDT/sbix export choices require RGBA strikes. |

BDF/PCF/PSF support gzip input and atomic gzip output. Standalone single-strike
exports require an explicit strike when the document has several; FON exports
all representable strikes. These formats cannot store a complete editable source
document; keep the `.fcf` alongside interchange exports.

EBDT/CBDT byte-sized fields constrain ppem/dimensions/advances to 255 and bearings
to signed bytes. Anisotropic embedded strikes, vertical-only small metrics and
16-bit-channel PNG are explicitly rejected. Library callers can opt into opaque
preservation with `ReadOptions { decode_bitmaps: false, .. }`; normal application
and CLI opens decode editable strikes.

## Rasterization, tracing and preview

`fc-raster` generates selected or all glyphs at a chosen size and depth without
modifying source outlines. Unhinted paths use tiny-skia; embedded instructions or
automatic grid fitting use skrifa on a temporary compiled font. The monochrome
threshold is configurable. Regeneration replaces the selected glyph images in
the matching `(ppem, depth)` strike and remains undoable.

Tracing creates exact closed pixel-edge contours, including holes, in the chosen
outline master. It updates outline advance and preserves anchors. It does not
perform photographic image recognition or curve smoothing; those remain P5 work.
The waterfall uses each real strike's pixels and advances, with a space for
missing characters. It does not shape OpenType layout or scale an outline as a
substitute for a missing strike.

Resource checks bound dimensions/ppem to 4096, each bitmap to 16 MiB, aggregate
pixel storage to 256 MiB, and a font to 256 strikes. References reject cycles,
excessive depth and excessive expansion work. Legacy format reads cap input at
128 MiB; PNG decoding, raster allocation and tracing also have work limits.

## Verification

The new tests in the working tree cover packed depths, padding, composite references, cyclic
and malformed input, transaction rollback, mixed history snapshots, native
persistence, stale-write conflicts, raster/trace pixel round trips, and source
outline preservation during grid fitting.

P3 adds 24 Rust tests, 5 frontend unit tests and 4 browser tests. The 934 Rust and
1,101 frontend totals below are whole-project regression totals, not newly added
P3 tests. The optional CJK Rust test also ran separately with its corpus enabled.

Browser acceptance in `app/e2e/bitmap.spec.ts` passed all four workflows:

1. Open BDF, draw, undo/redo, save FCF, export BDF and reopen saved pixels.
2. Inject a failed write, retain the draft, block navigation, retry successfully.
3. Trace, regenerate another strike, change properties, copy/cut/paste, delete
   and undo a strike, and render multiple waterfall rows.
4. Save metrics while the input retains focus, then draw an ellipse, invert,
   scale and crop.

Interactive agent-browser review used the real CJK corpus, searched for 中, and
inspected the editor in English/light and Chinese/dark. No browser exceptions
were reported. Screenshots were retained locally under `/tmp/fontcarver-p3-*`;
they and the external corpus are not redistributed as fixtures.

Independent interoperability checks used `bdftopcf` for four bit/byte/padding
combinations, FreeType through `fc-scan` for FNT/FON/OTB, and FontTools to decode
EBDT/CBDT/sbix output, PNG payloads and image origins. FontForge was neither
executed nor copied. Dependencies and specification sources are recorded in
[`THIRD_PARTY.md`](../THIRD_PARTY.md).

The optional `FC_CJK_BDF` integration test was run against the user's read-only
`PixelWorkshopSC-WorkBench-w320-16px.bdf`: **7,748 glyphs**, **546 ms** to import,
**62,363 ms** for the complete paged-preview and BDF/PCF/FCF/OTB semantic round
trip in a debug build, below the asserted 120-second budget. Every source bitmap
glyph's pixels and metrics were compared, not merely glyph counts.

### Reproducible checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace -- --skip licensed_real_font_inventory_reports_actual_coverage_without_mutation
cargo run -p fc-api --bin gen_ts
pnpm --dir app check
pnpm --dir app lint
pnpm --dir app exec vitest run --maxWorkers=2
pnpm --dir app build
cargo build -p fc-devserver
FC_API_PORT=8798 FC_UI_PORT=1438 pnpm --dir app e2e e2e/bitmap.spec.ts --workers=1
FC_CJK_BDF=/absolute/path/to/cjk.bdf cargo test -p fc-api --test bitmap optional_cjk -- --nocapture
```

The unrestricted workspace test run has one existing failure:
`fc-layout::licensed_real_font_inventory_reports_actual_coverage_without_mutation`.
For `NotoSerifTibetan-Regular.ttf`, layout decompilation claims support but the
generated source exceeds the feature class/rule expansion budget. The identical
failure was reproduced in a separate, clean checkout at pre-P3 commit `277fc5a`.
The remaining **934 workspace tests pass** with that one test excluded; this report
does not claim an unrestricted green workspace. Two existing tests remain ignored.
Rust formatting and workspace Clippy with warnings denied also pass.
The frontend suite passes **130 files / 1,101 tests**; typecheck, lint and production
build pass. Vite reports the existing advisory about chunks above 500 kB.

### CLI examples

```sh
fontcarver convert source.bdf edited.fcf
fontcarver convert edited.fcf output.pcf --ppem 16 --depth 1
fontcarver convert edited.fcf output.otb
fontcarver convert output.otb reopened.bdf --ppem 16 --depth 1
fontcarver convert console.bdf console.psf --psf1 --ppem 8 --depth 1
```

PSF2 is the default for `.psf`; `--psf1` requests PSF1 and validates its fixed-cell
constraints. Color embedded exports are selected in the export dialog or through
`session.export` with `Cbdt`/`Sbix`; normal `.ttf` CLI conversion embeds the model's
strikes automatically.
