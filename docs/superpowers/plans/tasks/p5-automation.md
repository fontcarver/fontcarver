# P5 automation implementation boundary

Owner: automation team; resumed 2026-09-30. Audit rows 79, 105 and 106.

## Scope and interfaces

- Add explicit local script discovery/reload and source loading through generated
  `automation.*` API methods, with bounded source, file and RPC limits.
- Expose the existing generated API contract to JavaScript macros through a
  stoppable worker and typed scripting declarations. Provide an accessible
  console with output/errors and local script selection; never run on reload.
- Run Python only on an explicit console action in a separate installed
  interpreter process. Bridge line-oriented bounded JSON RPC to the same API,
  reject recursive automation/process calls, enforce a deadline, and report
  unavailable interpreters and script failures. Python is trusted local code,
  not an operating-system sandbox.
- Provide a bundled curated plugin index, explicit install/enable/disable/remove
  lifecycle, canvas reporter overlay contract and revision-checked filter
  preview/apply workflow. No downloading or automatic execution of remote code.
- Document scripting types, lifecycle, trust and actual feature boundaries.

## Ownership

- `automation/backend`: `fc-api` automation module/types/tests and narrow API
  registry/generator additions, Python helper/runtime.
- `automation/console`: `app/src/automation` macro runtime and console UI/tests,
  narrow AppShell/command registration and `automation` English strings.
- `automation/plugins`: `app/src/plugins` contracts/manager/reporters/filter
  tests, narrow glyph canvas hook and `plugins` English strings.
- `automation`: integration, declarations/docs and browser acceptance evidence.

Coordinate shared registry edits with owners. No generated file edits, commits,
status/audit ledger edits or Chinese translations. All new dependencies require
permissive provenance; prefer existing dependencies.

## Acceptance

Test script discovery bounds and traversal, explicit execution and errors,
Python RPC calls/deadline/message limits, console cancellation and result
handling, plugin lifecycle persistence and reporter/filter cleanup. Confirm
filter preview is non-mutating, apply is undoable and rejects stale input.
Run focused Rust/frontend checks with the shared Cargo lock and task TMPDIR;
parent runs final broad integration. Run isolated headless agent-browser flows
for console, script reload and plugins, retaining current screenshots/report.
Run the artifact checker at start, around build batches and at completion.
