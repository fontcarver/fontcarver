# P2d-06: bounded FEA Cartesian expansion

## Scope and ownership

Close the independently reproduced Find problems/editor/compiler allocation
failures from compact ligature and enumerated pair-positioning class products.
Own `fc-compile/src/feature_validation.rs`, its new work-budget helper, and focused
`fc-compile/tests/feature_work_budget.rs`. No new dependency, public API, or UI
diagnostic code; reuse `feature-source-limit`.

## Approach

Inspect the parsed FEA tree before semantic validation/full compilation. Count
class entries and Cartesian rule work with saturating arithmetic without
materializing expanded glyph combinations. Preserve non-enumerated class kerning.
Run the same preflight at the existing direct compilation source boundary.

## Acceptance

- A 256-member three-component ligature and a 3000-by-3000 enumerated pair rule
  reject safely under a 512 MiB process memory limit, without compiler entry.
- Named aliases, nested classes, ranges, duplicate entries, and accumulated rule
  work cannot bypass the bound.
- Ordinary ligatures, enumerated pairs, and compact class kerning still compile.
- Existing feature validation, generation, writer, export, and source tests pass;
  format only owned files and run crate Clippy.

## Initial evidence

Independent API probes of both examples aborted allocating under 512 MiB;
the 256-glyph fixture triggered this in less than one second. Existing source
byte/statement limits and the new component-DAG budget do not catch FEA products.

## Result and verification

- Added source-ordered, saturating AST cardinality accounting for glyph classes,
  aliases, numeric/alphabetic/CID ranges, mark classes, ligature products,
  enumerated pairs, and aggregate work. Non-enumerated class kerning remains
  compact. Shared parsing returns the existing `feature-source-limit` diagnostic.
- Removed the marker-only shortcut at `FcSource`'s existing feature check, so
  direct static/binary compilation cannot bypass the preflight.
- Six focused feature integration suites: **30 passed**. The five work-budget
  regressions passed again after the final mark-class/range review fixes under a
  **512 MiB** address-space limit with one Rayon thread (**1.64 s**). The CID
  range unit regression also passed.
- Original API repro fixtures now return bounded error reports under the same
  limit: ligature **17 ms**, enumerated pairs **103 ms**, no allocator abort.
- Owned rustfmt check and diff check passed; crate Clippy passed, followed by
  the parent's final whole-workspace Clippy pass after the last fix.
- No dependency, translation key, or public API change; no FontForge execution
  or source adaptation. Parent owns final whole-workspace integration.
