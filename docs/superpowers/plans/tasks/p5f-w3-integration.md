# Wave 3 integration and final gate

Owner: parent orchestrator. Date: 2026-10-02.

## Scope

Coordinate the eight `p5f-w3-*` tasks, review their changes independently,
request separate GPT-6.1 Sol translation when English keys change, integrate
focused commits and update STATUS, TODO, the 112-row feature audit and progress
estimate with actual evidence. The parent owns these shared status documents.
Preserve bounded feature definitions; record any newly found ordinary defects
and fix them before claiming the relevant row complete.

## User constraints

The user authorized all remaining tasks and as many agents as useful. Later
steering explicitly defers online publication and confirms signing credentials
are not configured. Perform verifiable builds/install checks; do not publish,
invent signing credentials or mark unavailable native execution complete.
The latest 2026-10-02 steering revokes that temporary stop request: complete
all remaining local closing tasks, including integrated Linux installer build,
inspection, installation/removal and smoke checks, then update TODO/status/
progress. Publication and macOS/Windows execution remain outside the active batch.
Synthetic installer and minimal runtime probes do not substitute for real
FontCarver candidate acceptance.
No native/global desktop input automation is permitted. Both supplied remote
spellings (`fontcarver/fontcarver` and `fontvarver/fontcarver`) are currently
unresolvable for the authenticated GitHub account; do not create a replacement.

## Validation and resource coordination

All Cargo commands share `.scratch/p5f/cargo.lock` and default profiles.
Sequence heavy release builds and broad validation; check artifact budgets
around every batch. Reuse the shared target directory, and clean only verified
regenerable development artifacts if growth would cross the 20 GiB ceiling.
Run the final integrated fmt, workspace clippy/test, app check/lint/format/test/
build and default E2E once changes settle. Repeat only failed or materially
affected checks. Release manifests must identify the source revision and dirty
state accurately; no final package may silently represent an earlier tree.

## Acceptance

Record exact pass/fail/skip counts and platform limitations in
`docs/p5f-w3-integration-acceptance.md`. Each row receives an independently
reviewed status. Stop owned processes and clean scratch after preserving
necessary evidence. Commit completed work locally without publishing.

## Completion, 2026-10-02

The current local closing checklist is accepted. Integrated Rust/frontend/default
browser gates pass; actual `6e35f70` Linux deb and thin AppImage candidates pass
inspection, independent review, isolated APT install/reinstall/removal and
startup checks. A separate isolated virtual display verifies bounded GUI process
survival without input automation. All 112 rows were independently reviewed;
9 and 62 retain their native macOS/Windows gaps. See
[integrated acceptance](../../../p5f-w3-integration-acceptance.md) and
[TODO](../../../TODO.md). Final status/estimate/user documentation is synchronized;
resource closure and local commit state are recorded with the acceptance.
