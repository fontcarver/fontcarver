# Final audit rows 57–112

Task ID: p5f-w3-audit-b

## Scope and ownership
Read-only code and existing audit documents; own only docs/p5f-w3-audit-b.md and this task file.

## Interfaces and acceptance
Independently verify each of rows 57–112 against integrated source and cited evidence; cross-check Complete/Partial claims, actual workflow reachability, bounded limits vs ordinary missing functionality. Produce per-row table with evidence links and concrete defects, especially packaging/import/export/persistence/hinting. Do not simply repeat existing status. No full builds/tests, targeted read-only analysis and tiny scripts allowed; request necessary focused verification from parent. Root runs final integration gate.

## Shared execution rules
Read AGENTS.md, CONVENTIONS.md, docs/superpowers/plans/tasks/p5f-protocol.md and relevant audit/acceptance docs. The user explicitly authorized all remaining P5 tasks and parallel agents on 2026-10-02; historical “wave 3 not started” statements are superseded. This task is your scope. Shared checkout: no commits, push, stash, reset, clean or unrelated edits. Parent owns STATUS, TODO, feature audit, progress estimate, integration and commits. No recursive subagents needed.
Run artifact checker at start and finish, scratch only .scratch/<task-id>/ with absolute TMPDIR. All Cargo commands use flock .scratch/p5f/cargo.lock, default profiles and shared target. Tell parent before any heavy build or cleaning cache; parent coordinates broad validation. Never delete another task’s artifacts/processes or shared stores. Append gap analysis to this task file before implementation. Focused tests and exact outputs in own acceptance report, distinguish limitations and missing evidence. Final report under 300 words. Stop owned processes and clean owned scratch after preserving acceptance evidence. New translations require a separate GPT-6.1 Sol agent assigned by parent.


## Gap analysis

This is an independent source/evidence audit, not a feature implementation. Rows
57–61/63 have concrete export, profile and hinting paths; the real external
hinter still needs new execution evidence. Row 62 remains Partial until real
platform installers are installed and checked. Rows 64–112 carry implementation
and acceptance evidence, but the final report will distinguish supported
subsets from missing ordinary workflows and check each row's UI-to-kernel path.
The existing PNG/HTML/PDF browser-download failures affect current verification
of rows 71 and 109. Broad checks and any corrective implementation belong to the
parent or owning implementation agents. Publication and signing are explicitly
deferred by the user's latest answers.
