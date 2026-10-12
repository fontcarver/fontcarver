# P2a next step: horizontal kerning editing eligibility

Date: 2026-09-16. The user requested the next project step after the stored
kerning resolver. This plan covers one kernel slice; the full P2a workflow
remains in the [design draft](../specs/2026-09-07-p2-kerning-design.md).

## Interface and decisions

- Add `fc_doc::kerning::check_horizontal_editing(&Font) ->
Result<(), HorizontalKerningEditBlock>`. Reasons are `ImportedLayout`,
  `SourceOwnedKerning`, and `InvalidFeatureSource`.
- Imported-table authority always blocks authoring, even if FEA text is empty
  or the document was saved as FCF. Do not mutate source or tables.
- Parse the compiler's effective FEA with the already locked `fea-rs` 1.0
  parser. A parsed `kern` feature, including a feature using named lookups,
  an empty feature, or a conditional variation, reserves kerning for source.
  Comments and identifiers containing `kern` do not reserve it.
- Ignore disabled blocks exactly as the compiler does. Syntax/load failures
  block editing. Resolve source from memory only: external includes are not
  supported by the current in-memory compilation path and must fail closed.
- This is layout ownership and syntax checking, not full feature compilation
  validation, group validation, or proof of a final shaped advance. Pair/group
  commands and API handlers must call this check when they are implemented.
- Move the existing FEA assembly, unchanged, into
  `fc_model::Features::to_fea()`, shared by the compiler and eligibility check.
  This keeps `fc-model` parser-free and avoids a dependency from documents to
  the full compiler/shaper. No persistent fields or schema change.
- Add a direct `fea-rs` dependency to `fc-doc`: reuse the compiler's parser to
  establish layout ownership; its locally installed manifest declares
  MIT/Apache-2.0. No version upgrade or additional package is required.

## Work and acceptance

1. Write failing document tests for fresh/imported fonts, feature bodies and
   full blocks, prefixes/lookups, disabled blocks, comments/classes, malformed
   syntax, includes, conditional `kern`, and query purity.
2. Extract shared source assembly, implement the check, and pass those tests.
3. Verify compiler source compatibility by shaping a small font using prefix
   lookups, both stored class spellings, and disabled features.
4. Run formatting, focused crate Clippy, complete `fc-doc`/`fc-model` tests,
   and relevant compiler/preservation tests. Review the final diff and record
   results in `docs/p2a-acceptance.md` (section “Eligibility slice”).

## Following slices

Next: plan and implement validated pair set/remove commands, layout change
notifications and undo/redo, using this shared check. Then group transactions,
typed API queries/commands, compiler exception/persistence integration, glyph
inspector and text-gap controls, and translation/headless acceptance. Each
slice needs concrete interfaces and tests before implementation.

Performance investigations, FontForge comparison and native desktop automation
remain paused. This step adds no API endpoint or UI and does not complete P2a.
