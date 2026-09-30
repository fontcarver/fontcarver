# Feature tokens and target conditionals

The existing feature source editor now accepts a bounded FontCarver dialect.
Stored source, source IDs, notes and disabled blocks remain unchanged. Native and
source-format persistence therefore retain the editable tokens. Compilation and
validation expand a copy for the selected master and target.

## Supported syntax

- `$unitsPerEm`, `$ascender`, `$descender`, `$xHeight`, `$capHeight`, `$italicAngle`.
- `${A:width}`, `${A:verticalWidth}`, `${A:verticalOrigin}` use the selected
  master's glyph layer. Missing glyphs/layers/optional metrics are errors.
- `${-A:width / 10}`, decimal/exponent literals, parentheses, unary signs and
  `+ - * /`. Results must be finite, are rounded to integers and must fit i32;
  the FEA compiler additionally checks each destination's own numeric range.
- `$customNumber` reads numeric entries of the existing font or master
  `user_data.featureNumbers` object; master values override font values.
  Built-in metric names are reserved. No new persistent schema field is added.
- `$[name ENDSWITH '.sc']` emits an escaped space-separated glyph list; wrap it
  in brackets when a FEA class is needed: `[$[category == 'Letter']]`.
  String properties are `name`, `category`, `subCategory`, `script`,
  `productionName`. Operators are `==`, `!=`, `BEGINSWITH`, `ENDSWITH`,
  `CONTAINS`; `export` supports boolean equality/inequality. `AND`, `OR` and
  parentheses combine comparisons. Matching is case-sensitive; operator and
  boolean spelling is case-insensitive. Missing optional metadata compares as
  the empty string. Only exported glyphs are emitted, in font order.
- Whole-line `#ifdef VARIABLE`, `#ifndef VARIABLE`, `#else`, `#endif` select
  static versus variable compilation. Nesting is supported; each source item
  must balance its own directives. Unknown condition names, duplicate `#else`,
  `#elif` and missing/end-without-start directives fail visibly. Inactive code
  is not expanded. Comments, quoted FEA strings and escapes remain literal.

For example, this feature body produces a default-master constant in a variable
font and a master-specific constant in each static font:

```fea
#ifdef VARIABLE
pos A ${-A:width / 5};
#else
pos [$[name == 'A']] ${-A:width / 10};
#endif
```

This is not the complete Glyphs expression or predicate language. Regex,
arbitrary property traversal, scripts, condition expressions other than VARIABLE,
and automatically interpolating numeric token values are not supported.
The existing editor provides source editing; this task adds no numeric-parameter
management UI. Variable numeric tokens resolve against the design-default master.

## Bounds and diagnostics

Source and expanded source are limited to 1 MiB per item and 1 MiB expanded
content overall. Numeric and predicate expressions are limited to 16 KiB;
parentheses to 16 levels; numeric expressions to 256 nodes and predicates to
128 nodes. Conditional nesting is limited to 32 levels. Predicates allow at most
100,000 font glyphs, 1 MiB emitted names and 1,000,000 total glyph evaluations
across one assembly. Generated/copied source regions are bounded too.

Preprocessing failures identify the original source item and UTF-16 range.
Ordinary parser diagnostics after an expansion still map to original offsets.
Replacement text belongs to the source item as generated syntax. The static
compiler and variable compiler select their own branches; variable-font editor
validation also compiles the variable branch before source activation.

## Verification

Targeted `rustfmt --check` passed for all new modules/tests and owned integration
files. The first attempted model run failed during compilation because another
task was midway through `Layer.metrics_keys`; feature-token tests did not execute.
The subsequent model and compiler test commands waited for the shared Cargo lock
and were cancelled before Cargo started, at the integration owner's request to
unblock API generation. These runs are **interrupted, not passed**. The root
integration task owns consolidated verification:

```text
cargo test -p fc-model --lib features::
cargo test -p fc-model --test feature_tokens --test feature_assembly
cargo test -p fc-compile --test feature_tokens --test feature_source --test feature_validation --test feature_generation
```

Tests cover 15 numeric/predicate helper cases; six model assembly cases including
mutation, source mapping, conditions, malformed directives and authority; four
compiler cases for real shaping, both static masters versus variable target,
invalid variable activation and original Unicode diagnostic offsets. Runtime
acceptance remains pending the consolidated test results. Final task resource
check passed with build artifacts at 6.10 GiB and more than 668 GiB free.
