# P5 resumed hinting implementation — 2026-09-30

This extends the earlier [hinting checkpoint](p5-hinting-checkpoint.md).
The source changes below are implemented. The [central integration](p5-resume-integration.md)
subsequently executed the hinting regressions successfully; earlier pending notes
below describe the delegated handoff. Independent CFF1 private-dictionary
decoding/rendering also [passed](acceptance/p5-cff-private-independent.json).
This does not complete the broad hinting milestone or feature-audit rows.

| Area                       | Implemented behavior                                                                                                                                                                      | Evidence and remaining boundary                                                                                                                                                                                               |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CFF1 private dictionaries  | Preserves FD0 and additional private dictionaries, glyph assignments, normalized private values and emitted FDArray/FDSelect. Nonzero assignments remain meaningful even on empty layers. | [Codec report](acceptance/p5-cff-private-preservation.md). Runtime and independent rendering pending; CFF2 private variation data remains unsupported.                                                                        |
| Private-dictionary editor  | Select, edit and append dictionaries; assign glyph layers; revision checks, undo routing and FCF fields.                                                                                  | [Editor report](p5-cff-private-editor-acceptance.md): 5 frontend tests and focused ESLint passed; actual components inspected with mock transport. Rust persistence tests pending. Automatic analysis remains limited to FD0. |
| CFF flex                   | Rejects an authored flex flag whose segment and depth information cannot be represented.                                                                                                  | [Guard report](acceptance/p5-cff-flex-guard.md). Imported flex still becomes ordinary curves in the dependency's outline callbacks; full flex preservation is not claimed.                                                    |
| TrueType variable binaries | Variation-sensitive point bindings and instruction replacement preserving variation data; `cvar` requires unchanged CVT. Adds a guard against phantom-component binding bypass.           | [Binary checkpoint](p5-tt-variable-binary-checkpoint.md). Runtime tests pending. The compiler and editor still reject authored variable-font export.                                                                          |
| PS analysis                | Cross-section ink checks prevent a crossbar from making counter space score as a stem. Added candidate/work limits and dictionary guards.                                                 | [Analysis report](acceptance/p5-ps-analysis.md). Runtime tests pending. This remains a bounded geometry heuristic.                                                                                                            |
| Type 1 composition         | Imports representable terminal `seac` component hints with sidebearing/matrix placement and separate active masks. Rejects unsupported nesting and component-scoped counters.             | [Task acceptance](superpowers/plans/tasks/p5-type1-seac-hints.md). Five integration cases include independent FontTools/FreeType checks; centralized execution pending. No broad Type 1 mechanism completion is claimed.      |

The new authored-TT geometry option guards reject coordinate rounding, extrema
insertion and component decomposition before program source can be cleared.
Native Type 1 output rejects unresolved additional CFF private dictionaries or
nonzero assignments before replacing a destination.

All hint-owned queued Cargo jobs were cancelled at the root's request so one
integration owner can compile and generate API types. Initial tests encountered
concurrent compilation errors; cancelled or failed-to-compile runs are not passes.
Owned temporary work and browser/server processes were cleaned up. No FontForge
source, executable or new dependency was introduced.

Recommended audit state remains **Partial** for rows 61 and 98 until integration
evidence is recorded and the outstanding variable hinting, flex, dotsection and
general counter-control semantics are implemented.
