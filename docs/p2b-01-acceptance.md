# P2b-01 — mapped FEA assembly and UFO compatibility

Date: 2026-09-17. Status: implemented and independently reviewed.
Scope: [task 01](superpowers/plans/tasks/p2b-01-source-assembly.md).

## Result

Compilation, kerning ownership checks and UFO export now share one source
assembler through `Features::to_fea()`. Full UFO feature files are opaque
prefixes, so `languagesystem`, classes and lookups remain top-level statements.
Complete feature blocks retain comments/whitespace; body-only rules and `aalt`
feature references receive the appropriate wrapper. Class bodies retain their
spelling and trailing comments cannot swallow generated delimiters.

`Features::assemble_fea()` also returns byte mappings to stable source item IDs.
`locate()` maps valid UTF-8 positions to item-local byte and UTF-16 offsets;
generated wrappers map to their item without inventing a code position.
These are model helpers, not new API endpoints or an editor.

FCF stores source IDs with backward-compatible defaults, repairs missing/empty/
duplicate IDs, and narrowly migrates the historical lone synthetic `fea` block
to a prefix while retaining text, identity and disabled state. Imported authority
and other source arrangements are unchanged. The FCF schema records the rules.

UFO writing restores assembled text inside staging after Norad's newline
normalization, preserving CRLF, Unicode comments and exact spelling. Clearing
source removes a previous `features.fea`. Added only an internal `fc-compile`
test dependency to verify actual compilation/shaping; no third-party version
or runtime dependency was added.

## Regression evidence

Before implementation, the full-file UFO test failed with `languagesystem` in
an invalid feature block. Four model assembly tests failed on double wrapping,
`aalt` references and unwanted newlines. Native migration/ID and commented-kern
tests also failed before the change. A separate export regression reproduced
Norad's CRLF normalization. The final suites below pass these cases.

## Verification

| Command | Result |
| --- | --- |
| `cargo test -p fc-model -p fc-compile -- --skip source_han_export_meets_release_budget_when_available` | 84 passed: 45 model, 39 compiler |
| `cargo test -p fc-doc` | 118 passed, including 6 native-source persistence and 14 eligibility tests |
| `cargo test -p fc-formats` | 22 passed, including 4 source compilation/UFO roundtrip tests |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `cargo fmt --all -- --check` and `git diff --check` | Passed |
| `cargo run -p fc-api --bin gen_ts` and `pnpm check` | Passed; generated files remain ignored |

Total: **224 Rust tests passed**. The paused Source Han export timing test was
explicitly filtered. Actual compiled shaping checks cover ligatures, `aalt`,
kerning, full/structured source, duplicate tags, disabled blocks, and UFO
reimport. Source mapping tests cover Chinese/emoji, CRLF, generated regions,
reordering, EOF and invalid UTF-8 offsets. Independent read-only review found
no blocking implementation issue.

Local compiler/model, Clippy and generation logs are under ignored `target/`
with the `p2b01-` prefix. No frontend behavior changed in this slice, so browser
acceptance was not rerun then. Drafts, diagnostics APIs, imported conversion,
the Features editor and clean imported source-export policy were implemented
in subsequent tasks; see [final P2b acceptance](p2b-acceptance.md).

Next at this checkpoint was [P2b-02 — feature draft transactions and persistence](superpowers/plans/tasks/p2b-02-draft-transactions.md).
All P2b tasks are now complete; the current next milestone is P2c static OTF/CFF export.
