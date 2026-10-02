# macOS/Windows CI, signing and native evidence

Task ID: p5f-w3-native

## Scope and ownership
Own .github/workflows/ci.yml and .github/workflows/release.yml, new focused native smoke helper scripts/tests as needed, docs/p5f-w3-native-acceptance.md. Release/build/check_bundle scripts and install/releasing docs owned by packaging agent.

## Interfaces and acceptance
Inspect workflows thoroughly for execution defects, add meaningful platform-native checks to cover row 9/62 (build, install, launch, platform shortcut abstraction where automation allowed). No native/global desktop input automation. Determine available real macOS/Windows runner execution and signing capabilities; current gh account tomchen works but origin fontcarver/fontcarver unresolved, parent asking destination. Do not create/push repo, dispatch CI or publish until parent coordinates target. Prepare safe signing via existing CI secrets if provided, otherwise explicitly distinguish unsigned acceptance from signed release. Report precise external blockers while completing local script tests and workflow verification.

## Shared execution rules
Read AGENTS.md, CONVENTIONS.md, docs/superpowers/plans/tasks/p5f-protocol.md and relevant audit/acceptance docs. The user explicitly authorized all remaining P5 tasks and parallel agents on 2026-10-02; historical “wave 3 not started” statements are superseded. This task is your scope. Shared checkout: no commits, push, stash, reset, clean or unrelated edits. Parent owns STATUS, TODO, feature audit, progress estimate, integration and commits. No recursive subagents needed.
Run artifact checker at start and finish, scratch only .scratch/<task-id>/ with absolute TMPDIR. All Cargo commands use flock .scratch/p5f/cargo.lock, default profiles and shared target. Tell parent before any heavy build or cleaning cache; parent coordinates broad validation. Never delete another task’s artifacts/processes or shared stores. Append gap analysis to this task file before implementation. Focused tests and exact outputs in own acceptance report, distinguish limitations and missing evidence. Final report under 300 words. Stop owned processes and clean owned scratch after preserving acceptance evidence. New translations require a separate GPT-6.1 Sol agent assigned by parent.

## Gap analysis (2026-10-02)

- Row 9: one `Mod` abstraction and platform-dependent shortcut tests exist, but
  no native macOS or Windows execution evidence exists. Run those focused tests
  in each native runner and add installed-process startup checks. Process
  survival is explicitly weaker than rendered UI or interactive parity.
- Row 62: Linux smoke accepts any failing exit, including loader failures;
  macOS/Windows install steps do not start the installed application. Failed
  checks can also skip uninstall. A shared portable smoke helper will require
  the exact CLI version, reject crashes/loader failures, distinguish Linux's
  expected no-display refusal, and capture bounded native process survival on
  disposable macOS/Windows runners. Cleanup will use traps/finally blocks.
- Both `fontvarver/fontcarver` (literal user spelling) and
  `fontcarver/fontcarver` (origin) are unresolved by the authenticated GitHub
  account. The local host is Linux; Wine is not native Windows evidence.
  No push, dispatch, repository creation or publication is performed. The user
  confirmed signing credentials are not configured and requested no publication.
