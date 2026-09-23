# P5 MATH checkpoint

Status: paused at the user's explicit request on 2026-09-23, after the
import/export/API/editor integration step. Resume only when requested.
P4 is complete; P5 and MATH authoring are not complete.

## Model and codec step (2026-09-22)

- Font-wide named MATH model: all 51 adjustable constants, two minimum heights,
  three percentages, devices, italic corrections, top accent attachments,
  extended shapes, four-corner kern functions, both variant directions and
  assemblies. Glyph references use persistent names.
- Validation of finite rounded binary ranges, counts, references, device ranges,
  increasing kern intervals and a one-million-item resource budget.
- Standalone `fc_sfnt::math::{read_table,write_table}`. The reader supports both
  coverage formats and all three ordinary signed Device formats, checks
  parent-relative offsets and rejects malformed/unsupported input atomically.
  It limits input to 16 MiB and bounds repeated-offset amplification.
  The writer uses the existing write-fonts packer for Offset16 placement and
  deduplication; packing overflow is an explicit error.
- Additive `Font.math` and `Font.math_table_owned` fields, documented in the
  FCF schema. Legacy documents default to absent/unowned MATH.
- Glyph rename updates covered names and variant/part references. Rename
  snapshots restore MATH on undo/redo. Native FCF and gzip round trips verified.
- At that step, sfnt open/export still used the previous raw MATH preservation
  path (superseded by the integration step below). The new semantic codec is deliberately not activated in that path yet.
  An export preflight rejects explicitly authored/deleted MATH until integration
  is complete, preventing silent data loss or restoration of stale raw bytes.

No new dependencies, FontForge execution, copied FontForge implementation or UI
changes were introduced. Comments and documentation are English.

## Validation

Focused checks passed: 2 model tests, 7 binary-codec tests and 1 document test.
The codec tests include the committed DejaVu Sans MATH table, every constant,
glyph-order changes, coverage ranges, signed Device formats 1/2/3, invalid
offsets/devices, and malicious reuse of a large Device table.

FontTools 4.63 independently decoded and re-encoded the synthetic table,
checking constants, device corrections, kern values and both constructions.
OpenType 1.9.1 specifies `radicalDegreeBottomRaisePercent` as signed int16;
FontTools 4.63 still exposes it as uint16. The independent check compares its
exact 16-bit representation, preserving the negative source value on reimport.

Final checks:

- `cargo test -p fc-model -p fc-doc -p fc-sfnt -- --test-threads=1`:
  **292 passed, 0 failed, 0 ignored**.
- `cargo test -p fc-compile --test math_pending -- --test-threads=1`:
  **1 passed**, confirming authored MATH and explicit deletion fail clearly
  until pipeline integration is implemented.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `pnpm -C app gen:api` and `pnpm -C app check`: passed. Generated API files
  remain uncommitted.
- `cargo fmt --all --check`, Prettier on the four changed Markdown files,
  and `git diff --check`: passed.

The 10 focused tests above are included in the 292 regression tests, not
additional tests. No frontend behavior changed, so browser/UI regression was
not repeated for this standalone foundation.

## Integration step (2026-09-23)

Paused again at the user's request after finishing this step.

- **Open:** sfnt import decodes MATH atomically with the imported glyph names.
  Success yields owned editable data and removes the raw copy; failure keeps
  the raw bytes and reports `math-not-editable`.
- **Export:** `fc-compile/src/math.rs` writes owned MATH using the actual
  compiled glyph order after all other table rewrites. It covers TTF, OTF/CFF,
  WOFF/WOFF2, variable TTF/CFF2, static instances and production names. The
  temporary export preflight is removed. Explicit deletion stays absent and
  stale raw bytes are never restored. Table selection can omit MATH.
  References to non-exported glyphs are dropped from the output only, with a
  `math-references-dropped` warning. Invalid data fails the export.
  Legacy raw MATH keeps the old preservation policy.
- **Glyph set:** deleting glyphs removes their records, variants, and whole
  assemblies that use them. Duplicating a glyph copies its record. Both are
  undoable. UFO extras persist `math`/`math_table_owned`. Glyphs, SFD and
  Type 1 exports warn with `math-not-exported`; OTB omits MATH.
- **Model:** `MathTable::retain_glyphs` and bounded
  `MathAssembly::layout` (same repeat count for all extenders, interpolated
  overlaps, short-connector and part-limit reporting).
- **Document/API:** undoable `Command::Math(MathCommand)`: `Replace`,
  `SetGlobals`, `SetGlyph`, validated against the full glyph set.
  `Change.math` flag. API methods `math.state`, `math.apply`
  (revision-checked), `math.decode_imported` (for legacy raw tables, using
  provenance glyph order) and `math.preview` (variant/assembly/fallback
  choice with outlines and ink bounds).
- **UI:** `app/src/views/math/`: a MATH dialog from the Font Info toolbar.
  It has grouped constants with device corrections, filterable glyph records,
  italic/accent/extended-shape fields, four-corner kerning, variants,
  assembly parts and a live construction preview. Diffs are applied as one
  undo step and the session joins document save handovers. English strings
  are in `en.json`; zh-CN comes from a separate translation pass.

Verification for this step: core crates `fc-model/fc-doc/fc-sfnt/fc-formats`
416 passed; `fc-compile` + `fc-api` 394 passed with 2 pre-existing
release-only ignores; clippy and rustfmt clean. Frontend `pnpm check`, lint
and the full Vitest suite passed (1,131 tests) after merging the zh-CN
strings from a separate Sonnet translation pass. One full run hit a transient
`GlyphView` canvas-focus failure. That test is unrelated to this change and
passed in three isolated reruns and the next full run.

Localization review should confirm three terms the translator marked as
uncertain: “cramped” → 局促样式, “corner kerning” → 角点紧排 and
“stretch stack” → 伸缩堆叠.

## Resume here

1. Browser-verify the MATH dialog with agent-browser on DejaVu Sans: edit
   constants and a record, apply, undo, save and reopen, export, and check the
   assembly preview and validation messages. Add a component test for the
   dialog.
2. Independent checks: compare exported MATH with FontTools and, if
   available, HarfBuzz (`hb_ot_math_*` via uharfbuzz) for constants,
   variants and assemblies. Test a larger real math font such as Latin Modern
   or STIX Two Math by external path, checking time and memory.
3. Add user documentation (`docs/user/`) and mark the MATH task and audit
   entries complete only after steps 1-2 pass.

## Local working state

- Branch: `p2a-kerning-resolution`.
- Previous checkpoint: `9fc94b8` (MATH model and codec), following `cb87321`
  (Type 1 export preprocessing),
  `46092d4` (Type 1 hint replacement) and `ee30400` (static TT authoring).
- QA devserver port 8894 and Vite port 1494 were stopped for this pause.
  Isolated browser sessions had already been closed.
- Integration logs: `/tmp/fontcarver-p5-math-int-core.log`,
  `/tmp/fontcarver-p5-math-int-compile-api.log`.
- Earlier focused logs: `/tmp/fontcarver-p5-math-model.log`,
  `/tmp/fontcarver-p5-math-sfnt.log`, `/tmp/fontcarver-p5-math-doc.log`.
- Broad-check logs use `/tmp/fontcarver-p5-math-*.log`.
- Fast mode was not re-enabled; the last verified service tier was `default`.
