# T7 minor scoped review — 2026-09-07

Reviewer: Astra/root. Reviewed 4e76f97..10b612e, including the optional API projection, gesture cleanup, tool repaint, shape-dialog refs, modal guard, zoom-based pencil tolerance and fitter split/optimization.

The nearest-point estimate evaluates real curve points and retains the best squared distance including endpoints, the prior fit parameter and a coarse grid before bounded Newton refinement; a failed refinement can overestimate distance (more splitting), not underestimate the distance to the evaluated curve. Existing independent sampled cubic/quadratic error tests remain intact; new stationary/endpoint cases cover the numerical change. The split preserves the facade exports and existing corner/quadratic rules. Timing evidence from the implementer shows ~23–34% improvement on identical 201/501/1001-sample workloads; these are isolated profiles, not the separate T12 pointer-to-paint gate.

The API field is optional on the wire and in generated TypeScript, derives from existing model evidence and preserves actual outline-point precedence. No model schema or persisted font data changes. Dialog Enter and modal tool-key behavior have focused regressions. finishDrag cleanup now preserves a subsequent drag/nudge preview without unsafe return-in-finally.

Independent validation: Vitest 5 files / 95 tests passed (curveFit, editorController, ShapeDialog, useToolShortcuts, pathGeometry). Final full checks/API verification await the implementer report before merge.

Verdict: scoped changes approved subject to final verification. Integration residual: controller is 806 lines, slightly above 800 hard cap. Root assigns coherent extraction to the T8 integration agent (merged controller currently 852 lines), avoiding conflicting refactors. Preserve this task's preview guard through that integration. Zoom/Hand conformance evidence still needs the final report; no newly identified correctness blocker in reviewed code.
