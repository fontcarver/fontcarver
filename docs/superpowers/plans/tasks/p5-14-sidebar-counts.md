# P5 glyph inventory: sidebar counts

Scope: extend `doc.sidebar` with Unicode-derived subcategory glyph counts and a
complete encoded scalar inventory for language coverage. Existing category,
script, block and unencoded filter semantics remain based on the first assigned
Unicode value. Explicit glyph metadata does not change these filter counts.

Owned files: `crates/fc-api/src/types/font.rs`,
`crates/fc-api/src/handlers/doc.rs`, and
`crates/fc-api/tests/sidebar_counts.rs`.

Public interface: `SidebarCounts.sub_categories` is a defaulted list of
`[category, subcategory, count]` tuples. `SidebarCounts.unicodes` is a defaulted,
sorted, deduplicated list of every valid encoded Unicode scalar, including
secondary assignments. `SidebarCounts.tags` is a defaulted, sorted, deduplicated
list of editor tags present in the document for autocomplete. The added fields
are optional in generated TypeScript for older responses. None invent missing
Unicode repertoire data.

Acceptance: integration tests compare uppercase/lowercase counts with real glyph
filters, preserve first-encoding semantics, include secondary encodings once,
exclude invalid scalar values, and observe cache refresh after edits and undo.
Qualified category filters (`Letter/Other`) distinguish shared subcategory names;
their matching implementation is owned by the smart-filter agent.
Old sidebar JSON must deserialize with empty added fields. Write tests before
implementation. Coordinate focused Cargo runs, binding generation and broad
Rust checks with the root's shared build lock; use space-saving profiles and
check artifact budgets at task boundaries.

No dependencies, model/schema changes, translations or UI files are included.

## Verification handoff

Implemented the three optional TypeScript/defaulted Rust fields and four focused
integration tests. `rustfmt --edition 2021` on the owned Rust files and
`git diff --check --` on the owned paths completed successfully.

The first serialized `cargo test -p fc-api --test sidebar_counts` attempt could
not reach test execution while sibling `fc-model` changes were incomplete:

```text
error: could not compile `fc-model` (lib) due to 10 previous errors
```

The retry was cancelled before acquiring the build lock at the parent agent's
request; it is interrupted, not passed. Root integration owns the focused test,
generated bindings, full crate regression and Clippy acceptance. The qualified
category matcher is coordinated with the smart-filter owner.

Task scratch directory removed. Final resource check: build 5.96 GiB / 13,091
files; temporary 0.05 GiB / 5,367 files; free 669.40 GiB; `PASS`.
