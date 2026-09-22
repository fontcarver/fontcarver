# P5 hinting checkpoint — 2026-09-22

Status: the first hinting slice is implemented and verified. P5 and the complete
hinting milestone remain open. The complete exit audit is
[p5-feature-audit.md](p5-feature-audit.md); user instructions are in
[user/hinting.md](user/hinting.md).

## Implemented

- `fc-hint`: byte-preserving TT assembler/disassembler covering all 256 opcode
  bytes, bounded PS straight-edge/curve-extrema analysis, blue/stem inference,
  and an optional out-of-process ttfautohint bridge.
- PostScript editor in the glyph inspector and Font Info masters: stems, ghosts,
  replacement/counter masks, closing-edge masks, zones and private parameters.
  Preview is read-only; applying a draft is revision-checked and one undo step.
- CFF1 import/output of representable PS hints and private values. Missing hints
  due to unrepresentable transformations are reported. Multiple CID private
  dictionaries produce a distinct limitation warning.
- Static sfnt export options for overlap removal and automatic hinting, available
  in the UI and shared CLI. Sources remain unchanged. Topology-changing overlap
  removal checks retained point-dependent layout before returning an output.
- Native mask fields documented with backward-compatible defaults. Geometry edits
  invalidate hints on affected sources/components while preserving independent
  master and background layers; undo restores hints and geometry together.

## Verification

- Workspace Rust run: **1,010 passed, 2 ignored**, explicitly excluding
  `licensed_real_font_inventory_reports_actual_coverage_without_mutation`.
  This is the previously recorded Tibetan layout expansion-budget failure, still
  scheduled for P5 diagnosis. No unrestricted green workspace is claimed.
- Two later regression tests pass: curved-bowl stem detection and invalid active
  overlap masks. Latest focused compile validation passes **41 tests** across
  hinting, static exports and point-dependent layout preservation. There are
  **1,012 distinct passing Rust tests** across the workspace and later additions.
- Frontend: **1,118 tests passed in 137 files**. TypeScript, ESLint, Prettier,
  workspace Clippy with denied warnings, Rust formatting and production build
  pass. The existing large JavaScript chunk warning remains.
- Embedded Linux desktop build passes with the production assets (`cargo build
  -p fontcarver --features tauri/custom-protocol`). The desktop was not launched.
- Headless browser session `fontcarver-p5-hints20260922`: edit/apply/save/export
  and reopen a 70-unit stem, inspect the outline overlay, analyze/cancel and
  reopen without changing the source, export automatic hints and independently
  confirm the source remains at 70 while the exported stem is 67. The session
  was closed. Verified field-layout screenshots are
  `/tmp/fontcarver-p5-ps-editor-fixed.png` and
  `/tmp/fontcarver-p5-ps-fields-fixed.png`.
- `scripts/p5_hint_acceptance.py`: independent FontTools decoding and FreeType
  rendering for Source Sans/Serif CFF exports and a Noto Sans Latin subset.
  Authored 48-stem output exercises Type 2 stack splitting. Real ttfautohint
  1.8.4 creates **125 glyph programs** per TTF/WOFF/WOFF2 export, with unchanged
  coordinates, glyph mapping and horizontal metrics. **2,460 glyph/size renders**
  pass. Web containers are decoded with FontTools before rendering because the
  host FreeType cannot load WOFF2 directly. Their rendering digests match TTF.
  A separate browser-produced OTF adds **30 successful renders**.
- Process failure, invalid output and timeout/reaping are tested. The real external
  tool was extracted under `/tmp/fontcarver-p5-ttfautohint`, not installed globally
  or bundled. Evidence is under `/tmp/fontcarver-p5-hint-acceptance`.

## Remaining hinting work

Type 1 hint import/output; TT source-index binding, instruction/CVT/fpgm/prep
editing, authored-program export and hinted preview; variable hinting; multiple
editable CID private dictionaries; broader PS analysis quality and format-aware
handling of diagonal hints, active ranges and flex. These remain open and are
not represented as completed checklist rows. The other P5 categories have not
been completed by this checkpoint.

The implementation uses existing permissive dependencies and original code.
No FontForge source or executable was used. The optional external hinter is a
separate user tool; its upstream manual is linked in the user documentation.
