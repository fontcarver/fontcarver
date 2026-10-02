# Wave 3 independent release tooling review

2026-10-02. **Tooling review accepted after fixes.** No implementation blocker
remains in the reviewed native smoke/workflows, rootless Debian verifier,
AppImage inventory/gate or thin builder. This verdict is not acceptance of an
integrated installer that has not yet been built or installed. Final candidate
inspection/install/smoke evidence must be attached after the integration build.
The final Linux candidate review is now reopened following renewed user
authorization; the evidence below remains tooling/runtime evidence until that
review is recorded.

## Scope and evidence boundary

Read-only implementation review of native smoke/workflows, isolated Debian
installation, AppImage inventory/policy, and the proposed thin runtime builder.
No host installation, large build, desktop input, push, dispatch, signing or
publication was performed by this reviewer. Linux checks cannot establish
native macOS/Windows execution.

## Review rounds

1. Native descendant cleanup: the initial Windows cleanup used `taskkill` only
   while the parent process remained alive. An early parent crash could leave
   WebView2 children running. Fixed and reviewed: the wrapper joins an owned
   kill-on-close Job Object before launching the executable, aborts before launch
   if assignment fails, and retains its non-inheritable job handle until exit.
   Timeout cleanup directly kills/reaps the wrapper, eliminating the secondary
   `taskkill` timeout failure. The actual Windows orphan-child regression is
   present but cannot run on Linux.
2. Workflow coverage: the existing `scripts/*release*` PR filter and unit-test
   glob omit the new AppImage and Debian install helpers. Reported to the native
   owner and fixed: `scripts/**` triggers the workflow; Linux runs all script
   tests while macOS/Windows run the portable release suites.
3. Reused build output: bundle discovery can include older package versions
   alongside the freshly built candidate and does not compare them with the
   requested build version. Fixed and reviewed: candidate count/freshness checks
   reject unchanged or duplicate retained artifacts without deleting them.
   Source revision and a content fingerprint cover root dependency/toolchain
   inputs and start before license/API generation. A second review caught the
   initially omitted root pnpm lockfile and late snapshot; both are corrected.
   Build/bundle/install suites pass 33 tests. Rootless installation isolation and
   cleanup have no blocking review finding. Actual integrated candidate and
   install evidence remain pending. A subsequent helper fix was also reviewed:
   validated archive parents provide writable nested icon paths, `/proc` reflects
   the private PID namespace, and tempfile output plus owned process-group
   cleanup prevent retained stdout/orphan hangs.
4. Initial diagnostic inventory: no blocking defect found. File and runtime
   hashes are separate, escaped/broken symlinks fail, host package matches are
   explicitly candidates, and the inventory never grants release approval.
   The final gate was reviewed in round 6 below.
5. Native Open filter follow-up: independently accepted the nine added
   extensions in `nativeDialogs.ts`. Each matches an existing `session.open`
   codec branch; the focused tests cover the emitted filter and selected-path
   forwarding. This is a mocked Tauri boundary check, not a native picker run.
6. Thin AppImage: independently checked the actual runtime's eight pinned source,
   license and notice files; regenerated both source/linker patches in memory;
   inspected ELF dynamic dependencies and the compiler archive map. The complete
   source preserves its actual copyright notices. No blocking builder issue was
   found. Reported two integration issues: the policy gate initially omitted the
   builder's RPATH/general archive restrictions, and CI used Ubuntu 22.04 despite
   requiring squashfuse 0.5.0. Ubuntu's package records show
   [Jammy uses 0.1.103-3](https://packages.ubuntu.com/cs/source/jammy/arm64/squashfuse)
   while [Noble uses 0.5.0-2build1](https://packages.ubuntu.com/source/noble/squashfuse).
   Both were fixed and verified: the gate rejects RPATH/RUNPATH, unexpected
   archives and extracted archive members; source/input/link-map hashes and
   malformed receipt shapes are checked. The workflow uses Ubuntu 24.04, installs
   the required compiler/dev packages, and checks squashfuse 0.5.0 before the
   large build. No blocking issue remains in the reviewed recipe/tooling.
7. Host-library notice follow-up: inspected the final Linux payload gate requiring
   the host-library notice plus checksum-pinned LGPL 2 and 2.1 texts. Independently
   verified both repository text hashes, the updated thin README digest and its
   shared LGPL 2.1 pin. The thin recipe now includes nine approved input files;
   runtime source/binary behavior is unchanged by this notice-only addition.

## Checks performed

- `TMPDIR="$PWD/.scratch/p5f-w3-review-release/tmp" PYTHONPATH=scripts python3 -m unittest test_smoke_release_native test_check_deb_install test_appimage_inventory -v`
  — 20 tests passed in 0.628 s (initial implementation round).
- Parsed `.github/workflows/release.yml` with PyYAML and passed its five explicit
  Bash steps to `bash -n`: all passed. This is syntax review, not workflow or
  native execution evidence.
- `git diff --check`: passed at the initial review checkpoint.
- `TMPDIR="$PWD/.scratch/p5f-w3-review-release/tmp" PYTHONPATH=scripts python3 -m unittest test_smoke_release_native test_release_windows_job -v`
  — 13 tests run in 0.612 s: 12 passed, one real-Windows test skipped on Linux,
  after the native cleanup fix round.
- `TMPDIR="$PWD/.scratch/p5f-w3-review-release/tmp" pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 src/shell/nativeDialogs.test.ts`
  — one file, 13 tests passed in 1.71 s for the Open-filter follow-up.
- `TMPDIR="$PWD/.scratch/p5f-w3-review-release/tmp" PYTHONPATH=scripts python3 -m unittest test_build_release test_check_release_bundle test_check_deb_install -v`
  — 33 tests passed in 0.106 s after the packaging fixes, including the real
  synthetic Debian archive extraction regression (no host installation).
- `TMPDIR="$PWD/.scratch/p5f-w3-review-release/tmp" PYTHONPATH=scripts python3 -m unittest test_build_thin_appimage test_appimage_inventory -v`
  — 24 tests passed in 0.041 s in the initial thin-recipe review round.
- Actual runtime inspection: 49,720 bytes, SHA-256
  `7c017539a65934c5ffbe66a805c630323608794748d2a82cd5d88ef321c18f91`;
  eight source/license/notice pins and exact patch output verified. `readelf -d`
  records only `libc.so.6`, `libfuse3.so.3`, `libsquashfuse.so.0`, and
  `libsquashfuse_ll.so.0`; the link map passes the reviewed archive restriction.
- Start artifact check: 15.59 GiB build, 0.32 GiB temporary, 606.08 GiB free;
  passed with the 10 GiB build warning.
- Final gate/rootless-helper fix round:
  `TMPDIR="$PWD/.scratch/p5f-w3-review-release/tmp" PYTHONPATH=scripts python3 -m unittest test_appimage_inventory test_check_deb_install -v`
  — 24 tests passed in 0.035 s. A separate real child-process probe timed out
  after 0.1 s and verified after 0.5 s that its child could not write a delayed
  marker; normal command output was retained. The first probe expected the old
  `TimeoutExpired` contract while the owner added a controlled `RuntimeError`;
  it was corrected and passed against the final helper.
- Final workflow YAML parse and all five explicit Bash syntax checks passed;
  the Ubuntu 24.04 matrix and exact squashfuse prerequisite assertion were
  independently checked.
- Final artifact check and `git diff --check`: passed. Build 15.88 GiB / 19,640
  files, temporary 0.02 GiB / 2,107 files, workspace free 606.04 GiB; only the
  documented 10 GiB build warning remains. Reviewer-owned processes are stopped
  and `.scratch/p5f-w3-review-release` is removed.

## Remaining evidence boundaries

- The integrated application deb and thin AppImage still need their final
  actual inspection, install/removal and smoke evidence. A runtime-only probe
  and unit fixtures cannot establish these results.
- macOS/Windows package execution and Windows Job Object integration remain
  unverified on this Linux host; workflow definitions do not close audit rows
  9/62. GUI process survival itself is weaker than visible frontend readiness or
  interactive platform parity.
- The user deferred publication and has not configured signing credentials.
  Nothing was signed, pushed, dispatched or published by this reviewer.
