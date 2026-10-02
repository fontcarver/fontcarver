# AppImage payload license inventory and release gate

Task ID: p5f-w3-appimage

## Scope and ownership
Own scripts/check_release_licenses.py and test_check_release_licenses.py, scripts/release_licenses/*, new scripts/appimage_*.py and corresponding tests, docs/p5f-w3-appimage-acceptance.md, docs/licensing.md and narrow THIRD_PARTY.md license notices. Do not edit build_release.py/check_release_bundle.py/workflows: coordinate exact integration interfaces.

## Interfaces and acceptance
Inspect actual existing AppImage payload and current 73/10 claim. Make package/license inventory reproducible and account for all shipped files. Research authoritative license/source obligations, implement compliant source/notice/relink delivery if feasible; if bundling is incompatible with permissive dependency policy, design a concrete legitimate package strategy and an explicit fail-closed release gate. Do not call unresolved compliance complete. Coordinate packaging agent on interfaces, no broad release build until parent slot. License conclusions must cite official sources. Existing shared dependency stores and artifacts owned by others must not be deleted.

## Shared execution rules
Read AGENTS.md, CONVENTIONS.md, docs/superpowers/plans/tasks/p5f-protocol.md and relevant audit/acceptance docs. The user explicitly authorized all remaining P5 tasks and parallel agents on 2026-10-02; historical “wave 3 not started” statements are superseded. This task is your scope. Shared checkout: no commits, push, stash, reset, clean or unrelated edits. Parent owns STATUS, TODO, feature audit, progress estimate, integration and commits. No recursive subagents needed.
Run artifact checker at start and finish, scratch only .scratch/<task-id>/ with absolute TMPDIR. All Cargo commands use flock .scratch/p5f/cargo.lock, default profiles and shared target. Tell parent before any heavy build or cleaning cache; parent coordinates broad validation. Never delete another task’s artifacts/processes or shared stores. Append gap analysis to this task file before implementation. Focused tests and exact outputs in own acceptance report, distinguish limitations and missing evidence. Final report under 300 words. Stop owned processes and clean owned scratch after preserving acceptance evidence. New translations require a separate GPT-6.1 Sol agent assigned by parent.

## Gap analysis

Row 62 remains Partial. Source-package notice generation audits Cargo/npm, but
installer verification can currently say PASS for an AppImage without auditing
native libraries, generated launch scripts or its prepended runtime. The old
177-object / 126-package inventory flagged 73 **packages mentioning** copyleft;
that is not 73 established copyleft objects, and missing copyright data must not
be labelled permissive. Only ten package copyright files were shipped.

The retained image identifies runtime `AppImage/type2-runtime` commit `8f39b89`.
Its exact upstream license explicitly includes statically linked LGPL libfuse;
removing GTK/WebKit alone therefore does not make this runtime permissible under
the existing repository policy. Adding notices/source offers cannot override
that policy. Investigate a runtime with host dynamic libraries and a minimal
payload, retain full file/hash provenance, and reject release when provenance or
policy acceptance is missing. The concrete current policy-compatible Linux
format is the existing deb using distribution-provided dynamic libraries.
