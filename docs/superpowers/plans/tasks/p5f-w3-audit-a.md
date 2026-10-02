# Final audit rows 1–56

Task ID: p5f-w3-audit-a

## Scope and ownership
Read-only code and existing audit documents; own only docs/p5f-w3-audit-a.md and this task file.

## Interfaces and acceptance
Independently verify each of rows 1–56 against integrated source and cited evidence; cross-check existing Complete/Partial claims, actual workflow reachability, bounded limits vs ordinary missing functionality. Produce a per-row table with evidence links and any concrete defects; flag stale/unproved assertions. Do not simply repeat existing status. No full builds/tests, targeted read-only analysis and tiny scripts allowed; request necessary focused verification from parent. Check completed uncommitted E2E/metrics/wording code changes for correctness as relevant; root will run final integration gate.

## Shared execution rules
Read AGENTS.md, CONVENTIONS.md, docs/superpowers/plans/tasks/p5f-protocol.md and relevant audit/acceptance docs. The user explicitly authorized all remaining P5 tasks and parallel agents on 2026-10-02; historical “wave 3 not started” statements are superseded. This task is your scope. Shared checkout: no commits, push, stash, reset, clean or unrelated edits. Parent owns STATUS, TODO, feature audit, progress estimate, integration and commits. No recursive subagents needed.
Run artifact checker at start and finish, scratch only .scratch/<task-id>/ with absolute TMPDIR. All Cargo commands use flock .scratch/p5f/cargo.lock, default profiles and shared target. Tell parent before any heavy build or cleaning cache; parent coordinates broad validation. Never delete another task’s artifacts/processes or shared stores. Append gap analysis to this task file before implementation. Focused tests and exact outputs in own acceptance report, distinguish limitations and missing evidence. Final report under 300 words. Stop owned processes and clean owned scratch after preserving acceptance evidence. New translations require a separate GPT-6.1 Sol agent assigned by parent.

