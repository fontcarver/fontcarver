# P5 static SVG authoring acceptance

Verified on 2026-09-28 on Linux using the HTTP transport and isolated headless
Chromium. The static subset documented in [the user guide](user/svg.md) has an
editing path, undo, persistence and semantic OpenType export. This does not
complete COLRv1, general SVG graphics import, or the entire P5 checklist.

## Implemented behavior

- Shared SVG source is bound to persistent glyph names and XML element IDs.
  Import accepts plain UTF-8 and gzip documents; shared discontinuous binary
  ranges are decoded once. Export rewrites element IDs, fragment links and URL
  references to the compiled glyph order, preserving shared geometry and colors.
- Validation bounds bytes, XML nodes, nesting and reference expansion. Missing
  IDs, duplicates, external resources, cycles and unsupported content fail
  atomically. Unsupported imported tables retain their original bytes.
- Rename, duplicate, delete, undo/redo, FCF/FCF.GZ and UFO persistence are covered.
  Explicit deletion owns the output table, preventing stale imported data from
  returning. Formats that omit SVG issue a warning.
- Font Info includes source and binding editors in English and Simplified
  Chinese. Valid drafts flush on close/save; invalid bindings and XML remain
  available for correction. Preview uses isolated SVG image mode, with an
  explicit glyph selection and no execution of source markup in the UI DOM.

## Independent and browser evidence

`scripts/p5_svg_acceptance.py` generates original geometric SVG artwork using
FontTools 4.63.0, including gradients, opacity, local references, gzip compression
and discontinuous shared ranges. It independently compares parsed XML structure
and target identity after normalizing renamed IDs. All **8 exports** passed:
baseline and edited/renamed TTF, OTF, WOFF and WOFF2. Undo/redo and native
save/reopen also passed. Local report:
`/tmp/fontcarver-svg-qa/acceptance/report.json` (`complete: true`).

The browser exercised a missing glyph/element binding, failed close, correction,
source edits, Apply, Close, Undo, Redo, Save and TTF export. FontTools
independently confirmed the browser-exported purple artwork and its renamed
glyph binding. English/light and Chinese/dark
views were inspected. The preview now clears obsolete images immediately while
a replacement is being debounced, so a changed glyph label cannot display the
previous glyph's artwork. Screenshots contain only original artwork:
[English](screenshots/p5-svg-en-light.png), [Chinese](screenshots/p5-svg-zh-dark.png).
Both targeted WCAG 2 A/AA dialog scans reported **0 violations and 0 incomplete
checks**. This is not a full-application accessibility claim.

## Regression repair

The previously tracked Tibetan layout failure was reproduced: decompilation
reported eligibility even though its generated FEA exceeded the compiler's
class/rule expansion budget. The importer now calls the same compiler budget
check before offering conversion. The 1,182-lookup inventory remains readable;
conversion reports a source budget blocker and preserves the original tables.
The 13-test layout-import suite passed, including an explicit regression
assertion. No limit was raised and no test was skipped to obtain this result.

## Validation

- Rust regression run over fc-model, fc-sfnt, fc-doc, fc-formats, fc-compile,
  fc-layout, fc-api and fc-devserver: **1,032 passed, 0 failed, 2 previously
  ignored acceptance tests**. Optional external-corpus tests retain their usual
  availability behavior; this count is not a claim that every corpus exists.
- Final focused SVG tests include atomic/legacy APIs, every binary truncation,
  invalid ranges, gzip expansion, XML references, 20,000-glyph ordering and
  persistence/export. The large shared-document test passed its 10-second
  debug budget (whole six-test codec suite: 1.80 seconds in the final run).
- TypeScript check, frontend lint/build and affected-crate Clippy passed.
  Frontend tests: **142 files / 1,150 tests passed**, no unhandled errors.
  Rust formatting, changed-file Prettier and `git diff --check` passed.

The new `roxmltree` dependency is MIT OR Apache-2.0 and is recorded in
THIRD_PARTY.md. No FontForge implementation or third-party artwork was used.

## Boundaries

This verifies original static fixtures; real-world SVG font corpora and native
platform rendering are not claimed. The editor does not inject CPAL variables,
so SVG palette expressions use source fallback colors in preview. Grid/text
rendering stays monochrome. The editable subset excludes filters, raster images,
stylesheets, animations and foreign/external content; those imports remain raw.
The Chinese catalog was updated and checked for key/interpolation parity in this
pass; an independent translation review is still outstanding.
