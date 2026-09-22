# P5 MATH checkpoint

Status: paused at the user's explicit request on 2026-09-22, after completing
the current model and standalone binary-codec step. Resume only when requested.
P4 is complete; P5 and MATH authoring are not complete.

## Completed in this step

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
- Existing sfnt open/export still uses the previous raw MATH preservation
  path. The new semantic codec is deliberately not activated in that path yet.
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

## Resume here

1. Integrate semantic MATH decoding in sfnt open. On malformed input retain the
   raw table and emit a diagnostic; never return partial editable data.
2. Integrate the writer with actual compiled glyph order and table ownership.
   Replace the temporary export preflight once authored data and deletion work.
   Cover TTF, OTF/CFF, WOFF/WOFF2, production names, excluded glyphs, table
   selection and unchanged legacy raw preservation. Avoid misleading raw-table
   loss warnings once semantic ownership has taken over.
3. Finish glyph deletion/duplication policy and snapshots. Rename is covered,
   but deletion/duplication are not yet integrated with the new data.
4. Add revision-checked MATH API commands and complete undo/redo transactions.
5. Build the English/zh-CN editor and bounded assembly preview; test browser
   editing, validation failures, save/reopen, undo and export.
6. Expand real-font and independent HarfBuzz checks, then mark the MATH task
   complete only after the integrated workflow passes.

Full task: [p5-04-math-authoring](superpowers/plans/tasks/p5-04-math-authoring.md).
Remaining P5 work remains in the
[professional finish plan](superpowers/plans/2026-09-22-p5-professional-finish.md)
and [feature audit](p5-feature-audit.md). In particular, advanced hinting,
COLR/CPAL/SVG, geometry/construction gaps, comparison/merge, scripts/plugins,
remaining formats/profiles, shell UX, packaging and final acceptance remain open.
The previously known Tibetan layout expansion-budget test failure remains
unresolved; this checkpoint does not claim a passing full-workspace test suite.

## Local working state

- Branch: `p2a-kerning-resolution`.
- Previous checkpoint: `cb87321` (Type 1 export preprocessing), following
  `46092d4` (Type 1 hint replacement) and `ee30400` (static TT authoring).
- QA devserver port 8894 and Vite port 1494 were stopped for this pause.
  Isolated browser sessions had already been closed.
- Focused logs: `/tmp/fontcarver-p5-math-model.log`,
  `/tmp/fontcarver-p5-math-sfnt.log`, `/tmp/fontcarver-p5-math-doc.log`.
- Broad-check logs use `/tmp/fontcarver-p5-math-*.log`.
- Fast mode was not re-enabled; the last verified service tier was `default`.
