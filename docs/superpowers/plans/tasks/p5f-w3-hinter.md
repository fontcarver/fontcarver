# Real ttfautohint verification

Task ID: p5f-w3-hinter

## Scope and ownership
Own crates/fc-hint/src/external.rs, closely related fc-hint tests if required, scripts/p5f_real_hinter_acceptance.py if useful, docs/p5f-w3-hinter-acceptance.md. Do not change shared docs or install system packages yet: use an isolated extracted real distro binary or coordinate root for install.

## Interfaces and acceptance
Obtain/verify authentic real ttfautohint externally (never link/vendor/bundle GPL code), run conditional real-binary tests with unmistakable execution evidence, validate output with fontTools/FreeType/HarfBuzz as relevant, exercise failure/invalid-input behavior already covered. Investigate other ignored external hint tests. Fix real integration defects within scope. Record provenance/version and real pass counts; no skipped test represented as pass.

## Shared execution rules
Read AGENTS.md, CONVENTIONS.md, docs/superpowers/plans/tasks/p5f-protocol.md and relevant audit/acceptance docs. The user explicitly authorized all remaining P5 tasks and parallel agents on 2026-10-02; historical “wave 3 not started” statements are superseded. This task is your scope. Shared checkout: no commits, push, stash, reset, clean or unrelated edits. Parent owns STATUS, TODO, feature audit, progress estimate, integration and commits. No recursive subagents needed.
Run artifact checker at start and finish, scratch only .scratch/<task-id>/ with absolute TMPDIR. All Cargo commands use flock .scratch/p5f/cargo.lock, default profiles and shared target. Tell parent before any heavy build or cleaning cache; parent coordinates broad validation. Never delete another task’s artifacts/processes or shared stores. Append gap analysis to this task file before implementation. Focused tests and exact outputs in own acceptance report, distinguish limitations and missing evidence. Final report under 300 words. Stop owned processes and clean owned scratch after preserving acceptance evidence. New translations require a separate GPT-6.1 Sol agent assigned by parent.


## Gap analysis

Audit row 61 already validates options, runs an external executable without a shell,
checks output, captures warnings, and covers fake-process failures. Its sole real
binary test returns successfully when the program is absent; existing evidence
therefore proves neither actual invocation nor real output compatibility. Obtain
Ubuntu's authentic binary in task scratch, execute the bridge, and independently
check emitted font programs, shaping and rasterization. Extend real-binary
coverage only where it establishes meaningful behavior; absence must be an
explicit ignored test or a hard failure in the acceptance command. No new linked,
vendored or bundled dependency is needed. The two existing ignored workspace
tests are release workflow/performance tests in fc-api, not hinter tests.
