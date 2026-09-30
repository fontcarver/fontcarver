# P5 Rust/API integration

Scope: integrate the independently implemented resumed P5 features at shared Rust
and generated TypeScript boundaries. Feature owners retain their behavior and
tests. Do not overwrite concurrent changes or edit generated bindings by hand.

Ownership: integration-only constructor corrections, API registry/export
consistency, and regeneration through `fc-api`'s `gen_ts` binary. Coordinate each
shared-file change with its feature owner. Parent owns final regression tests,
frontend checks, status/audit edits and acceptance decisions. After the first
successful generation, the parent assigned this task workspace formatting,
workspace Clippy and one shared development-server rebuild.

Plan:
1. Review API method registration, dispatch, DTO exports and new model fields.
2. Run the generator under the shared Cargo lock; send domain failures to their
   owners and correct only agreed integration failures.
3. Refresh generated bindings after Rust compiles, notify frontend owners, and
   verify declared method types and imported generated files exist.
4. Report exact commands/results and resource checks without claiming unfinished
   domain features or platform acceptance.

Acceptance: generator completes successfully, API method/dispatch tables agree,
and generated contract imports resolve. Changes remain uncommitted. Run resource
checks at start, between build batches and before completion; keep temporary data
under `.scratch/p5-rust-integration/` and use the shared Cargo target directory.
