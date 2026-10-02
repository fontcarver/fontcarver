# Independent wave 3 release tooling review

Owner: independent review agent; read-only for implementation files.
Read AGENTS.md, CONVENTIONS.md, p5f-protocol.md and the implementation task files.
Own only this task file and docs/p5f-w3-review-release.md.

Review the new native smoke helper/workflows first (implementation ready), then
packaging/install and AppImage inventory/thin builder changes as their owners
announce readiness. Trace inputs, process cleanup, isolation, platform shell
semantics, archive/path safety, release gates, provenance and evidence claims.
Do not equate workflow syntax or mock tests with native execution. User deferred
publication and has no signing credentials. No code mutation, push or dispatch.

Report actionable defects with paths/lines and a plausible failure example to
parent and owning agent promptly. Verify fix rounds before final verdict. Tiny
focused tests are allowed with own TMPDIR/scratch, artifact checker at start/end;
no large builds or host package installation. End with an acceptance/rejection
verdict, residual native/signing limitations, and exact checks performed.

## Initial gap analysis (2026-10-02; superseded by the final verdict below)

- Native smoke now distinguishes the deliberate Linux no-display refusal from
  loader failure and reports GUI process survival without claiming UI readiness.
  Windows descendant cleanup after a parent crash needs independent checking.
- Rootless APT verification uses private user/mount/PID namespaces and overlay
  package paths; inspect cleanup/failure propagation and actual artifact identity.
- The diagnostic AppImage inventory remains explicitly non-approving and treats
  host package matches as candidates. The thin runtime and final release policy
  are still under implementation and need a separate final review round.
- macOS/Windows execution, signing and publication cannot be inferred from local
  Linux unit tests or workflow text. Publication is deferred by the user.

## Reopened final candidate review

The user reauthorized all remaining local closing work after the temporary stop.
Review the integrated Linux deb/AppImage source fingerprint, artifact and runtime
hashes, actual payload notices/policy gate, real rootless APT
install/reinstall/remove, and native smoke evidence after the parent freezes
and commits the source and the packaging owner builds the candidates. Native
macOS/Windows execution and publication remain outside this resumed scope.
Do not replace actual artifact evidence with mocks or change frozen product/tool
code without an observed defect. The reviewer continues to own only this task
file and its review report.

## Final verdict

Completed for the final frozen source `f6f6cf9`: actual deb/AppImage hashes,
source fingerprint, payload notices, nine thin-recipe pins, unique format
markers and the exact three-byte `DEB` to `APP` conversion were independently
verified. Real isolated APT installation/reinstallation/removal records and
final runtime/Xvfb records were checked against those same artifact hashes;
the reviewer also reran the final AppImage headless smoke. Accepted within the
explicit Linux evidence boundaries in `docs/p5f-w3-review-release.md`.
