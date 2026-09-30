# P5 glyph search and smart filters acceptance

Implemented 2026-09-30 under `p5-15-smart-filters.md`.

## Delivered behavior

Glyph search now supports All / Name / Unicode / Note scopes, case sensitivity,
and bounded regular expressions. Listing and index lookup intersect the same
query with existing filters, an exact case-sensitive glyph-name set, and a smart
expression. A present empty glyph-name set matches nothing. Sidebar categories
may qualify subcategories, e.g. `Letter/Other`, without matching other categories.

Smart expressions support `and`, `or`, `not`, parentheses, JSON-quoted literals,
comparison operators `:`, `=`, `==`, `!=`, `~`, `=~`, `>`, `>=`, `<`, `<=`, and
bare boolean properties. The visual editor uses the same parsed expression tree.
All requested predicate families are implemented, including compatibility,
hints, corners, annotations, metrics, kerning groups, colors and Hangul keys.
String comparisons in the DSL are case-sensitive; normal search has its own
case-sensitivity toggle. Scripts use model ISO 15924 values such as `Latn`/`Hani`.

Named collections use `Font.info.user_data["com.fontcarver.inventory"]` and the
existing SetFontInfo undo snapshot. The shared inventory helper changes only the
requested collection. Public methods are `glyph.smart_filters.get`, `.set`, and
`.parse`. No structural model changes or new document command were needed.

Input bounds include 16 KiB expressions, 2,048 lexical tokens, 16 tree levels,
256 tree nodes, 4 KiB literals, 1 KiB smart-filter regexes, and bounded 1 MiB regex
program/DFA allocations. Normal search has a 4 KiB query cap. Saved collections
are limited to 256 named filters and 1 MiB of text. Canonical text is revalidated
before persistence. `regex` is MIT OR Apache-2.0 and already present transitively;
the new direct dependency provides bounded linear-time matching.

## Evidence and remaining verification

Owned Rust files pass scoped rustfmt. Added query/parser unit tests, nine
predicate evaluation tests, five persistence tests, and a combined listing/index
integration test. Persistence tests cover ordinary/gzip FCF round-trip,
undo/redo, malformed-input atomicity and unchanged-save behavior. Predicate tests
include 20,000-glyph timing coverage. Cargo jobs were cancelled while waiting for
the shared lock under the user's final stop instruction; no Rust test pass is
claimed here. The root integration owner runs the consolidated suites.

Requested central coverage: `cargo test -p fc-api --test smart_filters --test
smart_filter_search --test glyph_list --test glyph_lists --test sidebar_counts`,
plus the `glyph_query`, `smart_parser`, and `smart_filter_eval` library tests.
Frontend focused test evidence is recorded separately in
`p5-smart-filters-ui-acceptance.md`. Full generated-type checks and interactive
visual verification belong to the central integration pass.

Layer colors, explicit case and Hangul keys use documented metadata fallbacks;
this task does not add importer projection or first-class authoring for those
properties. The evaluator honors canonical selected-layer editor annotations.
No FontForge source was consulted or adapted. No commit or push was made.
