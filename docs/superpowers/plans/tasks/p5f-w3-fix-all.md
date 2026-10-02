# Fix All complete deterministic repair batch

Task: p5f-w3-fix-all, authorized by parent under current user request.

## Scope and ownership

Own outline-audit FixAll implementation and focused tests (discover exact files and report parent first), docs/p5f-w3-fix-all-acceptance.md. Do not edit audit/status/shared en/zh catalogues unless coordinated.

## Interfaces and acceptance

Read row35 and current outline audit implementation plus /root/w3_audit_a finding: Fix All needs repeated press for multiple structural defect types on same contour and one extremum persegment. Determine actual ordinary expected behavior, add regression showing combined repairs should complete in one undoable batch, fix deterministic bounded convergence without risk of cycling/unboundedgeometry or overriding locked/read-only glyphs. Preserve invalid-input validation, one undo unit, existing scope of safe fixes (do not silently flatten or distort unsupported repairs). No broad tests; focused crate or UI checks, Cargo flock. Send exact limits and tests. Parent independent review afterimplementation.

## Protocol

Read AGENTS.md, CONVENTIONS.md, docs/superpowers/plans/tasks/p5f-protocol.md and relevant audit/evidence first. Append gap analysis before implementation. Shared checkout: no commits/push/reset/clean or others' edits, no nested agents. Parent owns integration/status, coordinates builds. Scratch only .scratch/p5f-w3-fix-all, absolute task TMPDIR, artifact checker at start/batches/end, Cargo via flock .scratch/p5f/cargo.lock with shared target/default profiles. No native/global desktop input automation. Preserve exact evidence in own report, stop ownprocesses and delete owned scratch after acceptance. User defers publication and has no signingcredentials; no publishing or inventednative evidence. Report under300words.

## Gap analysis

Row 35 currently reports all four checks and safely repairs one structural kind
per contour, but one Fix All press leaves ordinary combined defects and the
second extremum of a cubic unresolved. Single-Fix stale-snapshot checks, kernel
validation and path/glyph locks already exist and must remain.

Plan: retain safe per-pass planning, insert multiple extrema using descending
parameters rescaled after each split, then re-audit kernel results during one
bounded editing gesture. Disabled checks and locked paths stay excluded; cycles,
limits and failed commands must roll back this gesture without touching another
edit. Existing cancel is an unsafe end-plus-global-undo across asynchronous
handoffs, so a token-checked kernel cancellation endpoint is requested before
using repeated passes. General ambiguous coincidences remain manual review.

Owned frontend files: `app/src/views/glyph/outlineAudit.ts`,
`outlineAudit.test.ts`, `OutlineAudit.tsx`, `OutlineAudit.test.tsx`, new
`outlineAuditRepair.ts` and `outlineAuditRepair.test.ts`, plus
`docs/user/outline-editor.md` audit paragraph and this task/acceptance report.
Kernel cancellation ownership and a uniquely named E2E spec are coordinated
with the parent before edits.

## Approved ownership additions and completion

Parent approved token-checked cancellation in fc-doc/src/document.rs,
fc-api/src/{api.rs,handlers/doc.rs,bin/gen_ts.rs,handlers/automation/python.rs},
new fc-doc/tests/edit_cancel.rs and fc-api/tests/edit_cancel.rs. No API
parameter type was needed because cancellation uses existing EditEndParams.
The temporary undo.rs helper was removed; that file is unchanged. Shared
frontend additions are editor/editSession.ts and its direct tests,
test/mockTransport.ts, automation/protocol.ts and its runtime guard test.
app/e2e/outline-audit.spec.ts uses dedicated ports 8853/1463 and is coordinated
with the E2E owner. Parent delegated other cancellation caller test migration
to a separate agent. Three English audit strings were passed to the independent
GPT-6.1 Sol translator.

Implementation and scoped acceptance are complete; see
docs/p5f-w3-fix-all-acceptance.md. Parent owns broad integration and audit/status
updates. Reviewer findings about disabled-check caps, redo-tail preservation and
final event history labels are covered by regressions.
