# P2a kernel review and completion

Scope: independently review the P2a document commands, kerning queries, compiler
normalization and their regression tests. Preserve unrelated workspace changes.

Files: Rust sources/tests in `crates/fc-doc`, `fc-api`, `fc-compile` and
`fc-layout` touched by P2a. No frontend, translation, roadmap changes or native
desktop/performance automation.

Interfaces remain the commands and query methods specified in
`../2026-09-16-p2a-authoring.md`. Add preview diagnostics for records omitted
from compilation; preserve no-op transactions even with unrelated invalid
stored records. Check atomic undo, bounded query pages and compiler exceptions.

Acceptance: add failing regressions for discovered defects, run formatting,
Clippy and relevant Rust suites, and report exact results to the coordinator.

Independent read-only review may inspect document/API correctness and report
findings; implementation and final checks stay with the kernel reviewer.

## Completion report

- Fixed NaN-sensitive no-op snapshots and unchanged-invalid-data `SetMaster`
  metadata edits. Rejected glyph renames that would overwrite dangling source
  references. Added atomic `CreateKerningGroup` for UI create flows.
- Aligned canonical group validation, resolver rules, query diagnostics and
  compiler filtering. Invalid compiler records produce cached preview warnings.
- Reused export's verified imported-layout preservation in preview after a
  headless TTF-reopen regression. Current model metrics still win; unsafe glyph
  order disables imported layout with diagnostics.
- Added mixed-batch, generation pagination, non-exporting group, invalid-source
  persistence, GPOS/GSUB reopening and FCF atomic-failure regressions. Moved
  outline command options into a separate module; public interfaces unchanged.
- Independent document/API review found three defects; all were fixed and a
  second read-only review found no remaining blocker in that scope.

Final checks (2026-09-16):

```text
cargo test -p fc-doc -p fc-api -p fc-model -p fc-layout -p fc-compile -p fc-formats -- --skip source_han_export_meets_release_budget_when_available
412 passed; 0 failed; 2 ignored; 1 filtered out
cargo test -p fc-api -p fc-layout  # after imported-preview fix
fc-api: 133 passed; 0 failed; 2 ignored
fc-layout: 23 passed; 0 failed
cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 19.10s
cargo fmt --all -- --check
git diff --check
exit 0
cargo run -p fc-api --bin gen_ts
exit 0 (CreateKerningGroup bindings generated)
```

The latest suite totals are API 133, compile 37, document 111, formats 18,
layout 23 and model 92: 414 passed, with generated-binding tests included.
The two ignored tests are existing release acceptance/performance tests; the
filtered test is the paused Source Han release timing gate.

Finite unresolved UFO records survive UFO/FCF reopening and are excluded from
compiled layout with diagnostics. Non-finite imported kerning stays in memory;
native FCF save rejects it before touching the destination because JSON cannot
represent it losslessly. No wire/schema redesign was introduced. Native desktop
automation, release timing and FontForge comparison were not run.
