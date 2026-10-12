# P5 completion wave 2, task g: bitmap fonts

Rows 64–71 and 101–103 of the [feature audit](p5-feature-audit.md). Written
2026-10-01 against the `p5-color-authoring` working tree (baseline `5bd842e`),
following [the wave protocol](superpowers/plans/tasks/p5f-protocol.md). The
per-row gap analysis that this work was planned from is appended to
[the task file](superpowers/plans/tasks/p5f-g-bitmap.md).

Ports used: API 8807, UI 1507 (and 8808–8817 / 1508–1517 for repeated Playwright
runs, because each run wants a kernel of its own).

## What changed

**Kernel**

- `fc-raster`: `scale_bitmap(source, width, height, depth, filter)` with
  `BitmapScaleFilter::{Nearest, AreaAverage}` — the cross-strike rescaler, now a
  tested kernel function instead of a loop inside the API handler. Area
  averaging integrates the source over each destination pixel's exact footprint
  and premultiplies RGBA so a transparent neighbour cannot tint the result.
- `fc-model`: `validate_strike_property` and the limits
  `MAX_STRIKE_PROPERTIES` / `MAX_PROPERTY_NAME_CHARS` / `MAX_PROPERTY_VALUE_BYTES`,
  enforced from `BitmapStrike::validate`. A property name with whitespace or a
  control character, and a string value with a line break or NUL, cannot be
  written to BDF, PCF or FNT, so the model refuses them instead of letting the
  export fail later. Recorded in `fcf/schema.md`.
- `fc-doc`: **`BitmapCommand::SetGlyphs` bypassed the glyph lock.**
  `check_bitmap_locks` had arms for `SetGlyph`, `SetStrike` and `RemoveStrike`
  only, and the `SetGlyphs` arm of the bitmap `apply` reaches the strike arm
  _below_ the preflight, so a PNG sheet import overwrote a locked glyph. One
  additional match arm fixes it; see "Defects found" below.
- `fc-api`: `bitmap.grid` (new) returns every exact pixel grid for the
  document's units per em with the strike metrics it implies, plus the verdict
  and nearest exact size for a requested ppem. `bitmap.transfer` takes a
  `filter`. `bitmap.trace` takes `glyphs` and `all_images` and traces a whole
  strike in one undo step. `bitmap.pixel_components` validates a new pixel
  glyph's name.

**Frontend**

- `bitmapToolStore.ts` (new): the active pixel tool, the four canvas overlays
  (persisted in `localStorage`), and a count of mounted strike editors.
- `bitmapCommands.ts` (new): every pixel tool and overlay as a registered
  command in the Bitmap menu and the command palette, with a single-key
  shortcut gated on a strike editor being open.
- `BitmapCanvas`: Alt-drag stamps a copy of the marquee instead of moving it;
  the overlays come from the store; the HUD reports font units beside pixels.
- `bitmapRenderer`: the advance column is shaded behind the pixels.
- `BitmapGridPreset`: driven by `bitmap.grid`; a non-exact size is reported with
  the nearest exact one, and applying a preset sets the new strike's metrics.
- `StrikeProperties`: typed standard XLFD names, in-place value editing,
  rename, retype for custom names, the derived-property marker, and the same
  validation the kernel applies.
- `BitmapSheet` / `bitmapSheetImage`: the exported PNG carries its layout in a
  `tEXt` chunk, so re-importing it restores the cells, the columns and the glyph
  order; the layout is also shown as copyable text.
- `BitmapTransfer`: the rescaling filter. `BitmapWaterfall`: a single-glyph
  mode. `BitmapOutlineTools`: batch tracing and an "Edit the pixel glyph" route
  into the outline editor.

**Docs**: `docs/user/bitmap.md` (new), `crates/fc-doc/src/fcf/schema.md`.

## Commands run

Rust, each under the shared cargo lock with an absolute `TMPDIR`:

```text
cargo test -p fc-raster
  28 + 10 + 8 + 2 + 3 + 7 = 58 passed, 0 failed
  (lib; bitmap_scale; pixel_reconstruction; raster; trace_audit; trace_modes)
cargo test -p fc-model                    114 passed, 0 failed
cargo test -p fc-doc                      340 passed, 0 failed
cargo test -p fc-api --test bitmap --test bitmap_parity
  bitmap        4 passed, 0 failed
  bitmap_parity 17 passed, 0 failed
cargo clippy -p fc-raster -p fc-model -p fc-doc -p fc-api --all-targets -- -D warnings
  exit 0, no diagnostics
rustfmt --edition 2021 --check <16 owned files>   exit 0
flock … pnpm --dir app gen:api                    regenerated, no hand edits
```

Frontend:

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/bitmap src/canvas/bitmapGeometry.test.ts \
  src/canvas/bitmapRenderer.test.ts src/canvas/bitmapSheetImage.test.ts
  Test Files 16 passed (16)
       Tests 85 passed (85)

pnpm --dir app check
  Two errors remain in the whole app, both in `e2e/text.spec.ts`, which belongs
  to another wave-2 task. No bitmap file reports one.

pnpm --dir app exec eslint src/views/bitmap src/canvas/bitmap*.ts \
  src/shell/builtinCommands.ts e2e/bitmap.spec.ts            exit 0
pnpm --dir app exec prettier --check <owned files> src/i18n/en.json \
  ../docs/user/bitmap.md                     All matched files use Prettier style

FC_API_PORT=8817 FC_UI_PORT=1517 pnpm --dir app exec playwright test \
  e2e/bitmap.spec.ts --workers=1
  9 passed (33.5s)
```

The Playwright specs are the four P3 ones plus five new ones: pixel tools and
the marquee copy with persisted overlays; the exact grid preset and the
correction of a custom size; property rename and the sprite-sheet layout round
trip; cross-strike copies, batch tracing and the single-glyph waterfall; and
every pixel tool's single-key shortcut.

## Independent verification

`scripts/p5f_g_bitmap_acceptance.py` drives the live kernel over HTTP and
decodes everything it produces with fontTools, FreeType 2.13.2 and uharfbuzz:

```text
cargo build -p fc-devserver
target/debug/fc-devserver --port 8807 &
python3 scripts/p5f_g_bitmap_acceptance.py --out <scratch> --port 8807 \
    --system-freetype
  FreeType (2, 13, 2) from <scratch>/freetype-system/freetype/libfreetype.so
  ...
  34 of 34 checks passed
```

It covers, per row:

- **71** BDF import (strike identity, typed properties) and BDF/PCF reopening
  with every pixel intact.
- **102** EBLC advertising 8 ppem at 1 bpp in both the TTF and the OTB; EBDT
  pixels, bearings and advances equal to the BDF source; the OTB carrying no
  outlines; CBDT/CBLC and sbix written as PNG for an RGBA strike; FreeType
  rendering the TTF, OTB, BDF and PCF to the exact source pixels and decoding
  the CBDT and sbix strikes to the same coverage; HarfBuzz shaping from the
  embedded-strike TTF.
- **64** undo restoring a removed coverage strike beside an RGBA one.
- **69** the composite export referencing the `pixel` glyph, the pixel glyph
  travelling with the font, the decomposed export writing 16 plain contours, and
  FreeType rendering both identically.
- **103** DejaVu Sans not detected as a pixel font (0 evidence glyphs) and
  reconstruction refused.
- **67** FreeType rendering a rasterized 16 ppem strike exactly as stored, and a
  lower monochrome threshold keeping at least as much ink.

Two environment facts worth recording. The `freetype-py` wheel bundles a
FreeType built **without libpng**, so it cannot decode any colour bitmap and
reports `unimplemented feature`; the system `libfreetype.so.6` has PNG support
and was used instead (`PYTHONPATH` pointed at a copy of the package whose
bundled library is a symlink to the system one). Separately, FreeType reports a
font that carries **both** EBDT/EBLC and CBDT/CBLC as non-scalable with zero
units per em and serves it from the colour strike; that is FreeType's table
preference, not a defect in the written tables, and the colour strike renders
correctly.

## Browser evidence

Screenshots in `docs/screenshots/p5f-g-bitmap/`, taken headless through
agent-browser against `fc-devserver` on 8807 and Vite on 1507:

| File                    | Shows                                                                                                                                                                                                              |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `editor-overlays.png`   | The tool strip, the four overlay switches, `Advance 8 px (1000 units)`, `125 font units/pixel`, the shaded advance column, and the HUD mid-drag: `X 5, Y 1 px (625, 125 units) · ΔX 3, ΔY -2 px (375, -250 units)` |
| `grid-presets.png`      | `Integer pixel grid` showing `Custom (16 ppem)` with the message that 16 ppem gives 62.5 units per pixel and the `Use 20 ppem instead` button                                                                      |
| `strike-properties.png` | Each property with its type, `FONT_ASCENT · integer · written from the strike`, and the Edit / Rename / As string / Remove controls                                                                                |
| `sheet-layout.png`      | The sprite-sheet panel with the computed `16 × 8 pixels` grid, the export result and the `Sheet layout` description the PNG carries                                                                                |

The Bitmap menu was also inspected directly: it opens with **Bitmap editor**,
then the nine tools with their keys (`B E L R O G M I H`), then the four overlay
switches, each disabled with "Open a strike in the bitmap editor first." when no
strike editor is on screen.

## Defects found and fixed

1. **A PNG sheet import could overwrite a locked glyph.**
   `crates/fc-doc/src/command/editor_metadata.rs` checked bitmap locks for
   `SetGlyph`, `SetStrike` and `RemoveStrike`; `SetGlyphs` had no arm, and
   `fc-doc`'s bitmap `apply` forwards it to the strike arm _after_ the
   preflight, so the lock was never consulted.
   `locked_glyphs_block_every_bitmap_write_including_sheet_batches` in
   `crates/fc-api/tests/bitmap_parity.rs` covers all four commands plus
   `bitmap.transfer`, `rasterize`, `trace` and `pixel_components`; with the new
   match arm removed it fails on the first `SetGlyphs` write, which is how the
   defect was confirmed before the fix.
2. **An unrepresentable strike property reached the document.** A name with a
   space or a tab, or a value with a line break, was accepted by the model and
   only failed at BDF export. Now refused by `BitmapStrike::validate`, with
   `invalid_strike_properties_are_rejected_and_leave_the_document_intact`
   checking that the document and its history are untouched.
3. **A new pixel glyph could be given an unusable name.** `bitmap.pixel_components`
   checked only the length. It now applies the same name rule as the Add Glyphs
   recipe grammar when it has to create the glyph.
4. **The grid presets were a hard-coded list**, so 1024 and 2048 were missing
   for a 2048-unit em and 500 for a 1000-unit em. The list is now every exact
   divisor from 4 ppem up to the units per em, computed by the kernel.

A fifth defect was found and belongs to another task: while this work was in
progress `CloseDocumentDialog` called `useModalChrome` unconditionally although
it stays mounted with nothing to show, so `modalCount` never returned to zero
and **no keyboard shortcut in the whole application reached the command
registry**. It was diagnosed here (two P3 bitmap specs failed on `Mod+S` with
the bitmap commands disabled as well as enabled, which ruled this task out) and
task `m` fixed it with an `active` argument while this task was running. The
pixel-tool shortcut spec passes against the fixed shell.

## Per-row status

| Row | Recommended  | Evidence and limits                                                                                                                                                                                                                                                                                                                                                                                 |
| --- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 64  | **Complete** | Strikes keyed by `(ppem, depth)` with their own metrics and properties; duplicate keys, depth, size and negative metrics rejected without touching the document (`strike_identity_depth_and_size_are_validated_without_duplicates`); locks honoured by every write; `.fcf` reopen keeps metrics and properties; undo restores a deleted strike, verified through the live kernel.                   |
| 65  | **Complete** | Pencil (toggle-then-match), eraser, line, rectangle, ellipse, fill, marquee select/move and **Alt-drag copy**, whole-bitmap move, eyedropper and hand: all nine in the toolbar, the Bitmap menu, the command palette and on a single key, proved through the shell dispatcher (`bitmapCommands.test.ts`) and in the browser. Zoom is the toolbar's zoom level, a `<select>` with no key of its own. |
| 66  | **Complete** | Outline overlay, pixel grid, shaded advance column, pointer and drag delta in pixels **and font units**, units-per-pixel readout; all four switches persist across sessions and are commands. Screenshot `editor-overlays.png`.                                                                                                                                                                     |
| 67  | **Complete** | Per-glyph and whole-font rasterization with a threshold and optional grid fitting; threshold 0, depth 32, depth 3 and ppem 5000 rejected with the document unchanged; FreeType renders the result exactly as stored.                                                                                                                                                                                |
| 68  | **Complete** | Four corner modes and independent diagonal joining, batch tracing in one undo step, rectangle merging and collinear removal; every mode × join setting × four fixtures passes an outline audit (closed, finite, non-degenerate, no self-crossing, outer clockwise and holes the other way by nesting depth) and the model's geometry scan.                                                          |
| 69  | **Complete** | The `pixel` glyph drives every component; editing it halves every placed pixel without touching the composite glyphs; the export option keeps composites (with the pixel glyph in the font) or writes plain contours, and FreeType renders both identically.                                                                                                                                        |
| 70  | **Complete** | `bitmap.grid` returns every exact divisor from 4 ppem up to the em itself for the 1000, 1024 and 2048 unit families, with the strike metrics each implies; a non-exact size is **reported, not refused** — the control states what it costs and offers the nearest exact size, which applying creates the strike with (browser spec).                                                               |
| 71  | **Complete** | BDF import/export round trip (pixels, properties, bearings, advances) checked independently; a full property editor with typed standard names, in-place editing, rename, retype, removal and kernel-level validation; PNG sheets that carry their own layout, with a pixel-exact browser round trip.                                                                                                |
| 101 | **Complete** | The waterfall renders every strike for sample text and for the selected glyph alone, naming the glyph so an unencoded one is previewed too; cross-strike copy/scale with **nearest and area-average** filters, pixel-exact at the kernel and in the browser, one undo step, metrics and origins scaled.                                                                                             |
| 102 | **Complete** | EBDT/EBLC in TTF and OTB, CBDT/CBLC and sbix for RGBA strikes, all decoded with fontTools and rendered with a PNG-enabled FreeType, plus HarfBuzz shaping. Limits below.                                                                                                                                                                                                                            |
| 103 | **Complete** | Conservative detection and undoable reconstruction, exact on a TTF and an OTF round trip of a built pixel font, and no false positive on DejaVu Sans or Noto Sans (0 evidence glyphs, reconstruction refused).                                                                                                                                                                                      |

## Limits that remain

These are boundaries of the accepted behaviour, not unfinished work:

- **Colour strike exports take the whole font.** `Cbdt` and `Sbix` require every
  strike in the document to be RGBA; a document that also holds a coverage
  strike must drop it first. A normal `.ttf` export embeds both kinds at once.
- **A font with both EBDT and CBDT is served from the colour tables** by
  FreeType, which then reports it as non-scalable with zero units per em. The
  tables themselves are correct and each renders when it is the only kind
  present.
- **Pixel detection stays conservative** (ppem 4–256, three distinct staircase
  signatures, one exact orthogonal grid, no curves, effects or smart layers), so
  a pixel font drawn slightly off its grid is not reconstructed. That is
  deliberate: the alternative is reconstructing a font that is not one.
- **Area averaging into a 1-bit strike thresholds at half coverage.** It is
  offered because a deeper target is the normal use; the control says so.
- **The sprite-sheet layout chunk is FontCarver's own.** A PNG from another tool
  imports with the settings typed into the form, exactly as before.
- **Batch operations are bounded**: 4096 glyphs per trace, transfer or
  reconstruction, 256 glyphs per sheet, 16,384 pixel components per glyph and
  100,000 per transaction, 16 million pixels per rescale.
- **The `pixel` glyph is an ordinary exported glyph.** It has to be, because a
  composite that keeps its components needs its base in the font; exporting with
  "Expand components into contours" removes the need for it.

## i18n

New English keys, all under existing namespaces: `bitmap.strikeGone`,
`bitmap.waterfallGlyph`, `bitmap.waterfallGlyphEmpty`,
`bitmap.waterfallNoUnicode`, `bitmap.reason.noEditor`, `bitmap.property.*`
(including `bitmap.property.type.*` and `bitmap.property.error.*`),
`bitmapCanvas.{advanceColumn,fontUnits,unitsValue,positionUnits,deltaUnits}`,
`bitmapGrid.{choose,inexact,exact,useNearest}`,
`bitmapParity.{filter,filters.*,traceAll,editPixelGlyph,pixelMissing}` and
`bitmapSheet.{layout,layoutHint}`. Two existing strings were corrected:
`bitmapParity.transferHint` now describes both filters, and
`bitmapSheet.exported_*` no longer tells the user to remember the settings,
because the file now carries them. `zh-CN.json` was not touched; the
translation pass owns it.

## Context help

Task `m`'s `app/src/shell/commandHelp.ts` requires a help entry for every
registered command. All thirteen new bitmap commands (nine tools, four
overlays) already have one, with their `quickHelp.bitmap.*` descriptions, so
nothing needed adding here:
`pnpm --dir app exec vitest run src/shell/commandHelp.test.ts` — 8 passed.

## Shared files touched

Additive hunks only, each re-read immediately before editing:
`crates/fc-api/src/api.rs` (one method row, one dispatch arm),
`crates/fc-api/src/bin/gen_ts.rs` (four exports),
`crates/fc-doc/src/command/editor_metadata.rs` (one match arm, the lock fix),
`crates/fc-doc/src/fcf/schema.md` (one paragraph),
`app/src/shell/builtinCommands.ts` (one import, one call),
`app/src/i18n/en.json` (the keys above).
`app/src/api/generated/` was regenerated with `pnpm gen:api`, never edited.

## Dependencies

None added, in Cargo or npm. No FontForge source was read or adapted.

## Resources

`python3 scripts/check_artifacts.py` at the task start: build 6.16 GiB / 15,007
files, temporary 0.00 GiB / 0 files, 627.12 GiB free, **PASS**. At the end:
build 6.91 GiB / 15,708 files, temporary 0.04 GiB / 1,512 files, 624.70 GiB
free, **PASS**. It peaked at 19.53 GiB while six agents built into the shared
`target/debug` concurrently; another task cleaned the development profile before
this one finished.

This task's scratch directory, its ten `target/e2e-config-88xx` directories, its
`fc-devserver` (8807) and Vite (1507) processes and its named browser session
(`p5f-g-bitmap`) are gone. The retained evidence is this document, the four
screenshots in `docs/screenshots/p5f-g-bitmap/` and `docs/user/bitmap.md`.

## Review round

The review returned **accept with fixes**. Every item is below, with what
changed and the test that holds it.

**1 (blocking, row 71) — one locked glyph disabled the whole property editor.**
The `SetStrike` lock arm required every glyph in the incoming _and_ the stored
strike to be unlocked, and `bitmap.configure` writes metrics and properties
through `SetStrike`, so locking any glyph in a strike locked its metadata too.
The arm now requires an unlocked glyph only for the images the write actually
adds, removes or changes: a lock protects a glyph's pixels, not the strike it
sits in. `a_locked_glyph_still_allows_strike_metric_and_property_edits` locks
`A`, writes new ascent, descent and a `FOUNDRY` property, and asserts the write
lands, the locked image is untouched and one undo entry is added; it then
asserts that altering `A`'s advance or dropping `A` from the strike is still
refused with the document and its history unchanged.
`locked_glyphs_block_every_bitmap_write_including_sheet_batches` is unchanged
and still passes, so the lock itself did not loosen.

**2 (blocking, row 101) — the waterfall could not show an unencoded glyph.**
`BitmapPreviewParams` carried only `text`, so single-glyph mode was disabled for
`.notdef`, an alternate or the `pixel` glyph. It now also takes
`glyphs: Vec<String>`, which `handlers/bitmap.rs::preview` resolves by name and
prefers over the text, rejecting an unknown name and capping the selection at
128 like the text. `BitmapWaterfall` sends the name instead of spelling the
glyph, so the switch no longer needs a Unicode value and
`bitmap.waterfallNoUnicode` is gone. Covered by
`the_waterfall_previews_an_unencoded_glyph_by_name` (names win over text, an
unknown name is an error, a glyph with no image in the strike leaves a space,
129 names are refused) and by two frontend tests, including one that asserts
`.notdef` is selectable. `pnpm gen:api` was rerun under the cargo lock.

**3 (medium, evidence) — the verification script was scratch-only.** It is now
`scripts/p5f_g_bitmap_acceptance.py`, beside its siblings, with `--out`,
`--port` and `--system-freetype` (which points freetype-py at the system
library, the one with libpng). Its output is quoted under _Independent
verification_ above: **34 of 34 checks passed**.

**4 (low, docs) — the preset range and the word "rejected".** The cap is no
longer 512: presets run from 4 ppem to the units per em itself, bounded by the
largest strike the model stores, because below one font unit per pixel an exact
grid has no meaning. The 1000, 1024 and 2048 families therefore do get every
exact size, and the test asserts the full lists including 1000, 1024 and 2048. A
non-exact size was never refused and is not now: the control says what it costs
and offers the nearest exact size. The row 70 entry, "defects found" item 4, the
`bitmapGrid.inexact` string, the component's doc comment and
`docs/user/bitmap.md` all say that.

**5 (low) — `readPngText` could throw on a large chunk.** It spread a `tEXt`
payload into `String.fromCharCode`, which raises `RangeError` on a few hundred
kilobytes, outside `decodeSheetPng`'s `try`. It now decodes with
`TextDecoder('latin1')` and skips any chunk above `MAX_PNG_TEXT_BYTES` (64 KiB),
which is far above a 256-glyph layout. The new test feeds it a payload one byte
past the cap and asserts `null` rather than a throw, and one exactly at the cap
to show the walk still reads it.

**6 (minor).** The stale "dispatcher disabled" comment is replaced by a pointer
to the shortcut spec at the end of the file. `setup()` now calls
`dismissPrompts`, which answers a `Recover Documents` or `Unsaved changes`
dialog left by an earlier spec before anything else runs, so a modal overlay
cannot intercept the first click. Row 65's entry no longer lists zoom among the
single-key tools: it is the toolbar's zoom level, a `<select>`.

### Commands rerun for this round

```text
cargo test -p fc-api --test bitmap --test bitmap_parity   4 + 19 passed, 0 failed
cargo test -p fc-doc -p fc-model -p fc-raster             512 passed, 0 failed
cargo clippy -p fc-raster -p fc-model -p fc-doc -p fc-api --all-targets -- -D warnings
                                                          exit 0
rustfmt --edition 2021 --check <touched Rust files>       exit 0
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/bitmap src/canvas/bitmap*.test.ts src/shell/commandHelp.test.ts
                                     Test Files 17 passed, Tests 95 passed
pnpm --dir app check                 no error in any bitmap file
pnpm --dir app exec eslint <owned dirs and files>         exit 0
pnpm --dir app exec prettier --check <owned files>        clean
FC_API_PORT=8818 FC_UI_PORT=1518 pnpm --dir app exec playwright test \
  e2e/bitmap.spec.ts --workers=1                          9 passed (31.3s)
python3 scripts/p5f_g_bitmap_acceptance.py --out <scratch> --port 8807 \
  --system-freetype                                       34 of 34 checks passed
```

The `bitmap_parity` suite grew from 17 to 19 tests and the frontend from 85 to 95. No new dependency; the shared files touched in this round are
`crates/fc-doc/src/command/editor_metadata.rs` (the `SetStrike` arm) and
`app/src/i18n/en.json` (one corrected string, one removed key).
