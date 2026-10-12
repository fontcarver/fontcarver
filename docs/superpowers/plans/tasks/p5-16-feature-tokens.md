# P5-16 bounded feature tokens and target conditionals

Scope: compile the feature editor's stored token source without changing that
source, across static/variable export, validation and shaping. Row 100 only;
visual lookup authoring remains a separate task. No dependencies or schema fields.

Owned files: new `fc-model/src/features/{preprocess,numbers,predicates}.rs`, narrow
`features.rs`/`features/assembly.rs` exports and mapping changes, model integration
tests; narrow `fc-compile/src/{source,vertical,feature_validation,feature_generation}.rs`
integration and compiler tests. No frontend or shared progress documents.

Interface: `Features::assemble_fea_for(font, master, variable)` returns assembled
FEA or an item-local UTF-16 token error. Original `assemble_fea`/`to_fea` remain raw
source preservation APIs. Expanded source maps copied text back to original code;
replacement text belongs to its source item as generated syntax.

Supported bounded syntax: `$name` and `${expression}` for finite numeric metrics
and arithmetic; glyph metric references `A:width`; optional named numeric values
in existing `user_data.featureNumbers` objects (master overrides font). Glyph
predicates `$[name ENDSWITH ".sc"]` produce escaped glyph-name lists with explicit
property comparisons and bounded AND/OR/parentheses. `#ifdef VARIABLE`,
`#ifndef VARIABLE`, `#else`, `#endif` are whole-line directives, bounded nesting,
selected from the actual compilation target. Unsupported constructs fail visibly.
Comments, quoted FEA strings and escaped characters remain literal. This is a
bounded FontCarver feature dialect, not a claim of the complete Glyphs language.

Acceptance: tests first for source immutability, source/error offsets, bounds,
inactive invalid source, nested/malformed directives, number and glyph predicates.
Actual compile/shaping tests prove static versus variable branch selection and
metric/predicate effects. Run focused model/compiler tests using shared Cargo lock
and task TMPDIR; parent runs final workspace lint/regression checks. Artifact
checker runs before/after testing and before acceptance report.
