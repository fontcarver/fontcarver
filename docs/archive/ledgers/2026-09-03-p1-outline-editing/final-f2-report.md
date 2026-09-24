# Final review F2: ambiguous TTC preservation

Date: 2026-09-07  
Model: `gpt-5.6-sol`  
Branch/worktree: `p1-text-kernel` / `.worktrees/p1-text-kernel`  
Base: `04c1ceb`  
Commit: `37e39f7`

## Fix

`face_with_order` now returns original TTC bytes only when exactly one face matches the expected glyph order. A second match makes the original face unavailable, so the existing conservative path recompiles selected modeled geometry and uses the existing hinting-drop warning. No schema or provenance expansion was added.

The regression builds a two-face TTC in memory from the licensed `DejaVuSans.ttf` fixture. Face 1 changes an `A` coordinate from 700 to 737 while retaining original global hint tables; both faces have identical glyph order. Before the fix, untouched export copied face 0's 700 coordinate and instructions. After the fix, semantic point comparison retains face 1's 737 coordinate, output glyph instructions remain empty, and `hinting-dropped` reports unavailable outline bytes. No probe or binary fixture was committed.

## Verification

```text
cargo test -p fc-compile --test preserve
9 passed; 0 failed; 107.62s

cargo fmt --all -- --check
passed

cargo clippy -p fc-compile --all-targets -- -D warnings
passed
```

Only `crates/fc-compile/src/preserve.rs` and `crates/fc-compile/tests/preserve.rs` changed. Release binaries and final headless E2E must be rebuilt/rerun from the merged Rust tip.
