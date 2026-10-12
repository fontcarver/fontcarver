# P4 — Variable fonts and professional CJK

Status: implemented and verified on 2026-09-22 within the documented boundaries.
See [P4 acceptance](../../p4-acceptance.md) for final results and unsupported cases.

Follow-on authorization: after P4 is implemented and verified, continue directly
with the approved P5 scope. Do not substitute P5 work for unfinished P4 work.
The user requires final formatting and English code comments/developer documents.

## Binding scope

The P4 row of the approved design specifies axes/masters/instances UI, interpolation
preview, compatibility repair, brace/bracket layers, variable gvar/CFF2 export,
smart components, vertical metrics and vert/vkrn, Unihan filters/batch tools and
CID export. Its exit criterion is a variable Latin and CJK font built from masters
that exports and installs. Existing P3 work is preserved in its own checkpoint.

Do not mark this phase complete from implementation presence or aggregate test
counts alone. Record each requirement, its UI/API/export path, direct tests,
independent verification and any unsupported cases in a P4 acceptance report.
Do not substitute a static font for a requested variable font or discard source
variation data silently. FontForge is not executed or copied.

## Implementation order

- [x] 01 Design-space validation, coordinate mapping, multi-axis interpolation,
  and undoable axes/master/instance commands with native persistence.
- [x] 02 Multi-master fontc source and variable TrueType export, named instances,
  axis mapping, sparse/intermediate sources and configurable master subsets.
- [x] 03 CFF2 variable output and CID-keyed CFF output with independent decoding.
- [x] 04 Brace/bracket layer authoring and conditional substitutions in static
  instances and variable fonts; preserve these semantics across source formats.
- [x] 05 Smart-component axes, source layers, per-use values, resolution and
  export; bounded recursion and compatibility diagnostics.
- [x] 06 Axes/master/instance UI, live interpolation, named-instance previews,
  compatibility visualization and safe contour/start-point/order repairs.
- [x] 07 Vertical metrics, vertical text preview, vert/vrt2 and vertical kerning
  authoring/export; confirm vhea/vmtx/VORG and variable vertical advances.
- [x] 08 Unihan radical/stroke/reading filters, CJK batch actions, CID mapping and
  ordering, and the deferred reverse-cmap performance improvement.
- [x] 09 English/Chinese UI, focused regressions, actual variable Latin/CJK
  fixtures, independent sampling/shaping/install checks, browser and native
  checks where supported, final acceptance record.

## Verification

Use project-authored small fixtures to assert interpolation at default, endpoints,
interior and mapped locations; compare exported gvar/CFF2 results with model
previews using skrifa and independently FontTools/FreeType. Test incompatible,
malformed, duplicate and missing sources; verify undo/save/reopen and failure
recovery. Retain the P3 suite. Run Rust formatting/Clippy/workspace tests, API
generation, TypeScript/lint/unit/build and isolated browser workflows. Report
the existing P2 Tibetan layout-test failure separately if it remains unchanged.

## Working record

Implementation record (intermediate observations retained below; final results
are in the acceptance report):

- Validated mapped axes, full master/instance metadata and optimistic atomic undo.
- Multi-axis interpolation, sparse/intermediate sources, contour compatibility repair.
- Variable gvar and CFF2 export, named/static instances and source-master subsets.
- Conditional rvrn substitutions tested with shaping at both sides of a threshold;
  native Glyphs bracket read/write and source-layer authoring.
- Smart local axes/sources and component parameters, bounded nested resolution,
  model/preview/static/variable outline agreement and Glyphs/UFO persistence.
- Axes/master/instance editor, interpolation overlays, export options and English/
  Chinese strings. Smart component parameter editing and vertical editing were exercised in the browser.

Evidence gathered so far:

- Focused model, geometry, document, API, source-format and compiler tests pass.
  Workspace Clippy with all targets and warnings denied passed before the latest
  smart-axis duplicate-position/source-cloning edits; final checks remain required.
- Browser session `fontcarver-p4-20260922`: created a weight axis, a second master
  and a Medium instance; applied edits, selected the preview instance, exported
  variable TTF and CFF2 OTF. FontTools independently decoded both outputs.
  Screenshots and outputs are under `/tmp/fontcarver-p4-*`; the test source is
  `/tmp/fontcarver-p4-ui-source.fcf`. This small fixture does not establish CJK
  scalability, installation or native-desktop acceptance.
- Smart components use bounded rectangular local source grids. Missing corners,
  incompatible topology and out-of-range parameters fail explicitly. Glyphs pole
  sources use endpoints; FCF/UFO retain interior local sources.

Additional implementation and direct evidence:

- Vertical layer/master metrics, top-to-bottom preview and model-owned `vkrn`
  now work, including mapped-axis metric interpolation, group exceptions and
  separate manual-feature ownership. Compiler tests exercise TTF and CFF2.
- Unicode 17.0 Unihan embeds 102,998 records, preserving original readings and
  radical alternatives. Real JIS/KS/GB 18030 Level 1 and IICore repertoires are
  available in filters and Add Glyphs. Provenance and pinned source hashes are
  recorded in `crates/fc-glyphdata/data/README.md` and `THIRD_PARTY.md`.
- CJK filters and sorting, glyph metadata, captured-selection batch spacing,
  CID identity/mapping/numeric order and atomic undo are implemented. Tests
  check stale revisions, invalid ranges, same-encoding conjunction, explicit
  zero metrics, rename/delete references, failure rollback and native persistence.
- Reverse Unicode indexing is maintained incrementally through edits and history.
  A 65,536-glyph test checks 1,000 indexed width lookups within one second.
- Release API acceptance on 65,535 CJK glyphs: combined radical/stroke/reading
  search 51.7 ms; batch spacing preview 89.0 ms. Debug measurements were
  360.5 ms and 117.7 ms respectively. These tests use constructed glyphs.
- Browser session `fontcarver-p4-20260922` (closed): filtered radical 85 plus
  `shui`, inspected source `shuǐ`/`MIZU`, centered the captured glyph in both
  masters at advance 1000, vertical advance 1100/origin 900, and filled an
  Adobe-Identity CID map. Close applied the CID draft; Save persisted both
  changes in `/tmp/fontcarver-p4-cjk-ui.fcf`, independently checked from JSON.
  Evidence: `/tmp/fontcarver-p4-unihan-filter.png`,
  `/tmp/fontcarver-p4-unihan-metadata.png`, `/tmp/fontcarver-p4-cjk-batch.png`,
  `/tmp/fontcarver-p4-cid-editor.png`. The CJK UI fixture reuses simple project
  outlines; it is not the actual variable CJK font acceptance fixture.
- Rust workspace Clippy passed after CJK/CID/index integration. First broad
  regressions found two outdated expectations: Font Info was still asserted
  read-only, and group renaming was expected to leave dangling vertical group
  names. Both expectations were corrected; focused kerning tests (18) passed.
  Final broad regression and formatting checks remain required after subsequent
  changes. The known baseline Tibetan test is tracked separately.

The remaining items recorded during implementation were resolved by the final
verification work below. P5 follows this checkpoint and audits the broader
professional feature checklist, including the explicitly documented boundaries.

Final verification work (2026-09-22):

- Axis renames now migrate brace/bracket references atomically, including undo;
  partial tag typing retains the original identity. Smart and conditional source
  compatibility repairs preserve geometry and have direct undo tests.
- CJK centering resolves component dependencies before positioning composites;
  both selection orders and one-step undo are tested. Metrics edits now invalidate
  dependent glyph geometry. Finite transforms that overflow coordinates fail.
- Native designspace single-axis `rvrn` rules convert into editable bracket layers
  and export as real alternate glyphs plus rules. Generated aliases do not
  accumulate on repeated round trips. Unsupported rules remain lossless in source
  export and block interpolation/binary compilation explicitly.
- CFF2 validates malformed regions/programs, F2DOT14 representability, per-program
  65,535-byte limits and aggregate work. Static instances resolve optional master
  defaults and registered weight/width/italic/slant metadata.
- CLI build accepts `--variable`, `--instance INDEX` and repeated
  `--source-master NAME`. Actual real-outline master acceptance uses 53 Latin
  and 256 distinct CJK glyphs; all four TTF/CFF2 exports pass FontTools comparisons
  at 400, 475, 550, 625 and 700. The maximum observed boundary deviation is
  0.54846 units (CJK TTF cubic-to-quadratic conversion).
- Four variable faces were temporarily installed in the user's font directory,
  indexed/discovered by Fontconfig and then removed. FreeType rendered 640 samples;
  FreeType and HarfBuzz verified vertical variation at all five positions.
  Outputs, hashes and reports: `/tmp/fontcarver-p4-real-acceptance/`.
- Full Rust regression passed 993 tests with two previously ignored release tests,
  excluding the independently reproduced baseline Tibetan test. Subsequent focused
  tests cover two additional regressions. Final frontend regression passed 1,114 tests across 136 files. TypeScript,
  ESLint, Prettier and production/embedded desktop builds pass.
- The P3 browser regression passed four tests. Actual CJK Medium interpolation
  was visually verified in the browser. The glyph inspector now also offers a
  live, read-only interpolated view beside the editable master source.
- Embedded desktop compilation passed without launching a window. Previous user
  instructions prohibit desktop/global-input automation; this remains unverified.
