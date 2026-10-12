# P5 export geometry preprocessing

Scope: `fc-compile` export options, export-only geometry preparation, and focused
tests. Add extrema, whole-unit coordinate rounding, and component decomposition
to static exports after live geometry and instance resolution and before overlap
removal or hinting. Provide a shared helper for the Type 1 export caller. Reject
variable geometry processing explicitly and coordinate supported static CFF1
subroutinization with the subroutinization owner.

Plan:
1. Write tests for each transformation, source and unrelated-layer preservation,
   stale hint removal, unsupported options, and compiled output behavior.
2. Add a bounded preparation module and route the supported flags through the
   static export copy. Coordinate authored TT preflight with the hinting owner.
3. Run the focused integration suite using the shared Cargo lock and normal
   profiles, format owned files, and record the artifact check and test results.

Acceptance: flags change geometry when applicable, all original source metadata
remains unchanged, unchanged layers keep hints, unsafe options return descriptive
errors, and the compiled static outputs contain the prepared outlines.

## Implementation and verification handoff

Implemented the geometry flags and public `prepare_geometry_export` helper with
bounded outline validation, nested decomposition, extrema insertion, whole-unit
node/offset rounding, and dependent-glyph hint invalidation. Source documents and
unselected master layers remain untouched. Static CFF1 subroutinization is wired
to the separate subroutine owner's `otf` integration; other targets reject it.

Eight focused integration tests cover source preservation, unchanged-layer hints,
hidden component bases, nested components, invalid geometry, unsupported options,
and compiled OTF/TTF/WOFF/WOFF2 behavior. A ten-second helper budget is asserted.

Verification on 2026-09-30:

- `rustfmt --check --edition 2021 crates/fc-compile/src/export.rs
  crates/fc-compile/src/export_geometry.rs
  crates/fc-compile/tests/export_geometry.rs`: exit 0, no output.
- Initial locked `cargo test -p fc-compile --test export_geometry`: exit 101
  before these tests could build, due to concurrent missing `fc-sfnt::ttx` and
  incomplete `VariableStroke` matching in `fc-geom`.
- Queued retry was canceled following the parent's integration/stop instruction:
  exit 143, **interrupted, not passed**. `rust_integration` owns the final execution
  and broad checks; the focused suite is not claimed as accepted yet.
- Removed this task's empty scratch directories and stopped its queued process.
  Final `python3 scripts/check_artifacts.py`: `build: 6.07 GiB, 13,159 files;
  temporary: 0.03 GiB, 2,333 files; workspace free: 669.20 GiB; PASS`.

No additional dependency or FontForge source was used. Type 1 callers must
resolve live geometry before calling the shared helper; unresolved geometry
produces an explicit error.
