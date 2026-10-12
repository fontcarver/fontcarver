# P5 TrueType variable binary preservation

## Scope and ownership

Extend `crates/fc-sfnt/src/tt/` and focused integration tests with a bounded
binary-only variable-font instruction path. The editor/compiler continue to
reject authored variable exports until they can bind programs to the compiled
variable outlines. No model or API schema changes. Also extend the authored
program preprocessing guard in `crates/fc-compile/src/tt.rs` for the concurrent
outline-preprocessing options.

## Design

- Read default-instance stored points from existing `glyf` variable fonts.
- Include variation-space, outline-delta and metric-delta tables in a separate
  variable binding version; static binding digests remain unchanged.
- Rewrite instructions without altering variation tables or stored point order.
- Preserve `cvar` only with identical indexed CVT contents; reject changing or
  removing that CVT, orphan `cvar`, and invalid variation table headers.
- Do not infer validity at nondefault instances or claim interpreter safety.
- Keep phantom component anchoring explicitly unsupported in this slice.

Implementation is original, following the OpenType
[gvar](https://learn.microsoft.com/en-us/typography/opentype/spec/gvar) and
[cvar](https://learn.microsoft.com/en-us/typography/opentype/spec/cvar)
specifications. No FontForge source or executable is used.

## Acceptance

Write failing integration tests first. Verify successful variable program
replacement and exact variation-table/CVT retention; stale bindings after each
variation dependency changes; unchanged static digests; rejection of modified
CVT with `cvar`, invalid variation headers and malformed programs. Use focused
Rust tests and file-scoped formatting. Serialize Cargo with the resume lock;
parent runs final workspace Clippy and regressions. Record actual commands and
disk checks in the acceptance report.
