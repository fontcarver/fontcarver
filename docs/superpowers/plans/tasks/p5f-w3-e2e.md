# E2E browser downloads and external CJK acceptance

Task ID: p5f-w3-e2e

## Scope and ownership
Own app/e2e/*, app/playwright.config.ts, and a new docs/p5f-w3-e2e-acceptance.md. Existing isolation diffs are completed work: preserve them. Do not change product code or dependencies without coordinating the exact file with parent.

## Interfaces and acceptance
Resolve three bundled-Chromium Blob download cancellations without weakening assertions or skipping tests. Verify blank-page repro and investigate browser executable alternatives/version/environment. Enable real external CJK coverage from an existing/licensed font (large corpus stays external/scratch). Run focused failing tests first, then default Playwright suite when parent grants exclusive broad-test slot. Document exact browser/environment and counts. No full Vitest/Cargo workspace. Use API 8851 / UI 1461. Review initial isolation code as part of work; report any substantive defects before changes.

## Shared execution rules
Read AGENTS.md, CONVENTIONS.md, docs/superpowers/plans/tasks/p5f-protocol.md and relevant audit/acceptance docs. The user explicitly authorized all remaining P5 tasks and parallel agents on 2026-10-02; historical “wave 3 not started” statements are superseded. This task is your scope. Shared checkout: no commits, push, stash, reset, clean or unrelated edits. Parent owns STATUS, TODO, feature audit, progress estimate, integration and commits. No recursive subagents needed.
Run artifact checker at start and finish, scratch only .scratch/<task-id>/ with absolute TMPDIR. All Cargo commands use flock .scratch/p5f/cargo.lock, default profiles and shared target. Tell parent before any heavy build or cleaning cache; parent coordinates broad validation. Never delete another task’s artifacts/processes or shared stores. Append gap analysis to this task file before implementation. Focused tests and exact outputs in own acceptance report, distinguish limitations and missing evidence. Final report under 300 words. Stop owned processes and clean owned scratch after preserving acceptance evidence. New translations require a separate GPT-6.1 Sol agent assigned by parent.

## Gap analysis

The existing per-test kernel/config/font fixture removes cross-test document and
preference interference. The remaining recorded browser failures are three Blob
downloads (PNG, HTML and PDF); a blank-page Blob also fails under the bundled
headless shell, so browser/runtime selection must be investigated before touching
product download behavior. Installed full Chromium/Chrome executables allow a
controlled comparison using the same Playwright API and artifact assertions.
The external 65,535-glyph Source Han Sans case currently skips when its font or
release kernel is absent. Locate or obtain the licensed external font, verify its
identity, then run that test against the release kernel. Preserve download data
assertions and the existing performance limits. The final default suite requires
the parent's exclusive broad-test slot.
