# Native Linux rebuild/install and release tooling

Task ID: p5f-w3-packaging

## Scope and ownership
Own scripts/build_release.py, scripts/check_release_bundle.py and matching test files, docs/install.md, docs/releasing.md, docs/p5f-w3-packaging-acceptance.md. AppImage licenses owned by appimage agent; workflows by native agent.

## Interfaces and acceptance
Review current packaging tooling, fix real gaps, coordinate license gate with appimage agent. Prepare integrated-tree deb and AppImage candidate builds/inspection, actual apt install/remove verification in isolated disposable container/root (do not modify personal system packages without coordinating), headless binary smoke and manifests. Signing/publication require actual configured credentials and destination, no invented claims. Coordinate parent BEFORE heavy builds, cache cleanup or installs. Reuse target and serialize Cargo. No final release before integrated changes are ready.

## Shared execution rules
Read AGENTS.md, CONVENTIONS.md, docs/superpowers/plans/tasks/p5f-protocol.md and relevant audit/acceptance docs. The user explicitly authorized all remaining P5 tasks and parallel agents on 2026-10-02; historical “wave 3 not started” statements are superseded. This task is your scope. Shared checkout: no commits, push, stash, reset, clean or unrelated edits. Parent owns STATUS, TODO, feature audit, progress estimate, integration and commits. No recursive subagents needed.
Run artifact checker at start and finish, scratch only .scratch/<task-id>/ with absolute TMPDIR. All Cargo commands use flock .scratch/p5f/cargo.lock, default profiles and shared target. Tell parent before any heavy build or cleaning cache; parent coordinates broad validation. Never delete another task’s artifacts/processes or shared stores. Append gap analysis to this task file before implementation. Focused tests and exact outputs in own acceptance report, distinguish limitations and missing evidence. Final report under 300 words. Stop owned processes and clean owned scratch after preserving acceptance evidence. New translations require a separate GPT-6.1 Sol agent assigned by parent.


## Gap analysis (2026-10-02)

- Audit row 62: real Linux candidates exist only in a historical external target;
  the integrated tree has not been packaged or installed with APT. Rebuild the
  shared target after the parent freezes implementation changes, retain digests,
  and run real APT install/remove in a disposable user/mount namespace. The host
  has no passwordless sudo or Docker daemon, but unprivileged overlay mounts
  were proved available without changing host packages.
- The existing bundle inspector covers executable headers, desktop integration,
  dependency notices, and mandatory Debian copyright. AppImage native payload
  license clearance is a separate interface owned by the AppImage agent.
- `docs/install.md` still says the desktop has no `--version`/`--help`, although
  current source implements both plus a typed no-display error. Update those
  claims against the rebuilt binary and use the native agent's smoke helper.
- Native macOS/Windows installation is owned by the native agent. Signing and
  publication are explicitly deferred by the user's latest instruction; no
  credential creation or public release is in this task.

Parent extended ownership to `scripts/check_deb_install.py` and
`scripts/test_check_deb_install.py` for reproducible rootless install/removal.
The probe initially exposed a kernel-created mode-000 overlay work directory;
that owned probe was repaired and removed immediately, and the resource checker
passed again. The helper must normalize its work directory during use and always
clean it after its child namespace exits.
