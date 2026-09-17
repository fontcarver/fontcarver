# P2b — design audit and independent review

Date: 2026-09-17. Status: design audit complete; subsequent implementation tasks
01–11 are also complete (see [P2b acceptance](../../../p2b-acceptance.md)).

Read `CONVENTIONS.md` and the binding P2/product specs before working.
This task prepares the detailed P2b design; it does not implement features.

## Parallel audit scopes

1. **Imported layout audit** (read-only): inspect import, model provenance,
   compilation/preservation, shaping and the locked fontations/FEA APIs.
   Recommend the conversion ownership boundary, lookup coverage, lossless
   kerning-lift criteria, unsupported cases and semantic acceptance tests.
   Cite repository files and primary format/compiler sources where needed.
2. **Editor/source audit** (read-only): inspect feature model, FEA assembly,
   document commands/undo/persistence, API, editor shell and edit barriers.
   Recommend concrete source editing/diagnostic interfaces, draft lifecycle,
   automatic features and independently implementable slices. Identify missing
   infrastructure and dependency/license decisions without changing code.
3. **Final design review** (read-only, after the draft exists): check the new
   spec, plan and task files against code and the binding P2 scope. Report
   actionable correctness gaps, missing dependencies and unverifiable exit
   criteria; do not widen scope to P2c/P2d or paused investigations.
4. **Checkpoint audit** (read-only): the user separately authorized the
   coordinator to decide and create local commits. Inspect the dirty diff and
   prior acceptance records. Recommend a safe split between pre-existing
   rename/licensing/preferences work, completed P2a and new P2b documents.
   Identify mixed files and dependency risks; do not stage or commit anything.

## Files and interfaces

Auditors may read the repository and locally installed dependency source.
Only the coordinating agent edits design/plan/task/status documentation.
No production interfaces are changed by this audit.

## Acceptance and report

Return findings with file references, proposed boundaries and concrete tests;
distinguish inspected facts from design proposals. No Rust/frontend test run
is required for read-only audit or documentation edits. The coordinator records
the review outcome in the implementation plan after resolving findings.
Auditors do not commit. The coordinator may create user-authorized local
checkpoints after reviewing changes. No release, performance study, FontForge
comparison or native desktop automation is part of this task.

## Outcome

Imported-layout and source/editor audits were completed independently. A third
audit established the rename/P2a/design commit split. Independent final design
review findings and their fixes are recorded in the
[implementation plan](../2026-09-17-p2b-editable-layout.md#review-record).
The coordinator verified the isolated rename tree and unchanged P2a production
hashes before creating the first two local commits. No P2b runtime code was
implemented by this audit.
