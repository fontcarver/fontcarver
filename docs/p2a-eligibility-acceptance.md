# P2a horizontal kerning eligibility acceptance

Date: 2026-09-16. Branch: `p2a-kerning-resolution`; changes are uncommitted.
Scope: [bounded plan](superpowers/plans/2026-09-16-p2a-kerning-eligibility.md).

## Result

Implemented `fc_doc::kerning::check_horizontal_editing`: imported layout,
source-owned `kern`, and unparseable source produce distinct blocking reasons.
The parser handles prefixes, lookup references, conditional features, comments
and disabled blocks. Includes fail closed without reading files. Checks leave
source and stored data unchanged.

Moved existing FEA assembly into `Features::to_fea()`, shared with compilation.
The existing class-spelling regression moved with it. A new export/shaping test
covers prefix lookups, legacy/bracketed classes, contextual substitution and
disabled invalid source: both `AV` and substituted `AA` shape to advances
520 + 600 with the source pair value -80.

Dependency justification: `fc-doc` now directly uses the already locked
`fea-rs` 1.0.0 parser (`MIT/Apache-2.0`); no new external packages or version
changes. No FontForge source was used.

## Verification

| Command | Result |
| --- | --- |
| Initial `cargo test -p fc-doc --test kerning_eligibility` | Expected failure: missing new interface |
| Same command after implementation | 13 passed |
| `cargo test -p fc-doc -p fc-model` | 92 document + 37 model tests passed |
| `cargo test -p fc-compile --lib --test feature_source --test preserve` | 4 unit + 1 source/shaping + 9 preservation tests passed |
| `cargo clippy -p fc-doc -p fc-model -p fc-compile --all-targets -- -D warnings` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `git diff --check` | Passed |

Final diff reviewed for unchanged assembly, parser error handling, dependency
scope and preservation of pre-existing edits. No independent review was run.

## Boundary and next step

This checks layout ownership and syntax, not complete FEA compilation validity.
Future authoring queries and commands must enforce it. No API/UI or mutation
commands were added; P2a remains incomplete. Next: validated horizontal pair
set/remove, layout invalidation and undo/redo. Native automation, performance
investigation and FontForge comparison remain paused.
