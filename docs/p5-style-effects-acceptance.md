# P5 batch style effects acceptance

Status: complete within the boundaries below, verified on 2026-09-28.
Task: [P5-08](superpowers/plans/tasks/p5-08-style-effects.md).
Usage: [Batch style effects](user/style-effects.md).

## Delivered scope

- Filled-outline anisotropic weight expansion/contraction; width/height scaling,
  oblique shear and translation; eight editable presets including small caps and
  sub/superscripts. Presets transform selected outlines in place.
- Captured selection and master, optional all-master operation, bounded API
  preparation, geometry work outside the session lock, and revision-checked
  atomic apply. One undo restores all outlines, anchors, advances and backups.
- Original-source component resolution prevents double transformation when a
  selected composite references a selected base. Backups contain resolved drawing
  geometry and anchors/advances and survive native save/reopen.
- Optional advance scaling and post-effect centering for fixed CJK cells.
- English/Chinese dialog, Font menu/command palette and grid toolbar entry,
  invalid numeric text handling, before/after outlines and preview invalidation.
- Strict booleans are checked against an independent polygon engine. Recoverable
  failures use bounded curve refitting, with every affected glyph/master named in
  the preview. Unrecoverable errors reject the complete batch.

Two real defects were exposed and fixed during acceptance: capped open strokes
were not explicitly closed before filled booleans; and narrow closed stroke
rings could invert when the weight exceeded the stem width. Weight now unions
open boundary-segment capsules after removing internal overlap. Independent
sampling exercises both original and exported filled regions. The final two-input
operation checks symmetric-difference area against the polygon engine and refits
if necessary. Validation rays avoid exactly coincident horizontal boundaries,
where FontTools point-in-outline classification is ambiguous. Working coordinates are only enlarged for anisotropic
weight, so numeric tolerances do not grow with the requested offset.

## Verification

- `cargo test -p fc-geom -p fc-api -p fc-doc -p fc-compile`: **658 passed,
  0 failed, 2 pre-existing ignored**. Kernel cases cover anisotropic counters,
  full contraction of thin strokes, overlapping source regions, curved offsets,
  quadratic output, Noto Sans refitting, invalid/open paths and affine anchors.
  API cases cover selected base/composite batches, all masters, CJK centering,
  generation checks, late-error rollback, undo/redo, native persistence and export.
- `cargo clippy -p fc-geom -p fc-api -p fc-doc --all-targets -- -D warnings`
  and Rust formatting are clean.
- `pnpm -C app test`: **144 files / 1,156 tests passed**. TypeScript, ESLint,
  production build and changed-file Prettier checks pass. The existing large
  JavaScript bundle warning remains (about 1.31 MB uncompressed).
- `scripts/p5_style_acceptance.py`: FontTools 4.63.0 independently decodes **32
  outputs** (eight presets × TTF/OTF/WOFF/WOFF2), verifying bounds, advances,
  counters, component decomposition, native backup persistence and undo/redo.
- `scripts/p5_style_real_fonts.py`: DejaVu Sans and Noto Sans, eight representative
  glyphs each at +10 and −10 units: **19,212 filled-region samples agree** after
  export. Near-boundary points are excluded within 1.5 font units to account for
  quadratic approximation and integer rounding. Preview took **0.329–0.528 s**
  per eight-glyph batch in the recorded run. Noto Sans S reports refitting in both
  directions; the other seven glyphs use the strict path.
- Isolated Chromium: menu/toolbar entry, captured A/B selection, invalid-number
  rejection, explicit preview, anisotropic weight and fixed-cell centering, apply,
  Save As, native reopen and independently decoded browser-authored TTF. Chinese
  dark-theme preset/preview and English light-theme UI were inspected. Targeted
  whole-dialog axe 3.5.6 WCAG 2 A/AA checks: **0 violations, 0 incomplete** in each.
  Screenshots: [English/light](screenshots/p5-style-effects-en-light.png),
  [Chinese/dark](screenshots/p5-style-effects-zh-dark.png).

The generated synthetic fixtures and reports live under `/tmp/fontcarver-style-qa`;
they are reproducible and are not committed font assets. The capped-stroke fix is
`eae0f60`. Full frontend testing also exposed a test teardown race in SVG preview
URL mocks, corrected independently in `d3c3796`.

## Boundaries

Only selected main master outlines/anchors and optional advances are authored.
Components are decomposed. Backups preserve drawing content rather than the full
layer metadata or hint programs; Undo restores the complete source. No model or
FCF schema extension is required. Small-cap/subscript/superscript presets do not
create alternate glyphs or OpenType feature rules. Weight supports same-sign
nonzero offsets on both axes, and no weight on open contours. Curve refitting is
explicit in the preview. Variable master compatibility must be checked after
weight edits. This milestone does not close the non-destructive effects, smart
geometry, tracing, optical spacing or complete P5 acceptance blocks.

No new runtime dependencies or FontForge source/execution were used. Native or
global desktop input automation was not used. Independent Chinese translation
review remains outstanding; the strings were authored in the same implementation
pass because delegation was unavailable under the active execution constraints.
