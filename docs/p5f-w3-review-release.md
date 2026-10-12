# Wave 3 independent release tooling review

2026-10-02. **Final Linux candidates and release tooling accepted within the
scope below.** The independently inspected deb and thin AppImage were rebuilt
from frozen source `f6f6cf9`, including the corrected AppImage format marker.
Their actual payloads, source fingerprint, notices and runtime evidence pass;
the real isolated APT install/reinstall/remove evidence also passes. No reviewed
implementation blocker remains. Native macOS/Windows execution, visible GUI
interaction, signing and publication are not established by this verdict.

## Final actual candidate review

The build [manifest](acceptance/p5f-w3-packaging/release-manifest.json) and
[log](acceptance/p5f-w3-packaging/build.log) identify revision
`f6f6cf9ce481d11314e25c540b5719fd0951a00f`, 2,008 input files and input SHA-256
`2fe41daf9aa27d48230fb3ac970ab119867c49245f2dcdff47017c60c93a6624`.
These exactly match this reviewer's checkpoint before the build and independent
checks before and after the final extraction review. The evidence copy of the
manifest also equals the one beside the actual artifacts.

| Actual candidate                  |      Bytes | SHA-256                                                            |
| --------------------------------- | ---------: | ------------------------------------------------------------------ |
| `FontCarver_0.1.0_amd64.deb`      | 24,235,798 | `e01096f3abd387e5758dc177cfe5a85e4ec5030613d1f3f1c890aac789e429de` |
| `FontCarver_0.1.0_amd64.AppImage` | 23,683,640 | `3a9029b1aa83d39d78365b18202b39aa5a035f640083dfb4e53151c205793536` |

Both actual files were independently hashed and extracted into reviewer-owned
temporary directories. Fresh `inspect_payload` and
`require_releasable_appimage` results exactly matched the preserved manifest.
The deb has 1,145 files and the AppImage 1,160; each contains notices for 630
packages in 1,126 notice files. All seven project notices, including the
host-library notice and checksum-pinned LGPL texts, match current source bytes.
The bundled `INSTALL.md` also matches its source. The AppImage has one payload
ELF and no bundled shared libraries. All nine approved source/license/notice
pins, both regenerated patches, the runtime hash, dynamic dependencies, link
map and receipt were verified against the actual extracted image.

Every deb file except the application ELF has the identical hash in the image:
1,144 unchanged files. The actual application ELF changes only three bytes at
offset 4,743,271, from its unique `DEB` marker to its unique `APP` marker.
Reversing those bytes exactly reconstructs the deb ELF. Its SHA-256 is
`9715a2ad24ecaa9975cea03a56cf68ffe2de95931a569884b900000980fa976e`;
the image ELF is
`c8fe34032a54e4e609ae0e641f0c149becc67dbef66c2086d1059187066f9432`.
The raw release executable also matches the deb executable after Tauri's
expected `UNK` to `DEB` marker conversion. The receipt binds the image to the
actual deb hash above.

The [actual APT record](acceptance/p5f-w3-packaging/deb-install.json) and
[full execution log](acceptance/p5f-w3-packaging/deb-install.log) were reviewed
independently. They show installation, same-version reinstallation, package
triggers, removal and private document/preferences/recovery retention in the
isolated namespace, using this exact final deb hash. Both installed executable
smokes report version `0.1.0` with exit 0 and the deliberate no-display
diagnostic with exit 1, without timeout. The host executable remains absent.
The namespace's nonfatal `root:adm` log ownership warnings did not prevent APT
completion. This is a review of actual owner-run installation evidence, not a
claim that this reviewer repeated the installation or that a clean OS resolved
the dependencies.

The reviewer separately ran `smoke_release_native.py --launch headless` against
the actual final AppImage using the thin owner's isolated host-library sysroot.
Version `0.1.0` exited 0 in 0.573 s; the expected GTK no-display diagnostic exited
1 in 0.465 s, with neither probe timing out. The owner's preserved
[runtime probes](acceptance/p5f-w3-thin-appimage/candidate-probes.json),
[build receipt](acceptance/p5f-w3-thin-appimage/BUILD.json) and
[link map](acceptance/p5f-w3-thin-appimage/runtime.map) agree with the new artifact,
application and deb hashes. They additionally establish normal FUSE launch,
extract-and-run launch and complete extraction. The unchanged runtime is
49,720 bytes with SHA-256
`7c017539a65934c5ffbe66a805c630323608794748d2a82cd5d88ef321c18f91`.
The [isolated Xvfb record](acceptance/p5f-w3-thin-appimage/gui-probes.json) binds
the new artifact hash and source revision to 15.065 s of native process survival
before controlled SIGTERM. It contains no desktop input and does not establish
visible frontend readiness or interactive behavior.

## Superseded first candidate and format-marker correction

The first integrated candidates from `6e35f70` passed independent extraction,
payload/notice checks and runtime smoke. Their manifest source fingerprint
matched the reviewer's pre-build checkpoint. Real APT install/reinstall/remove
and user-data retention evidence was also reviewed.

A final executable provenance comparison found one actual metadata defect: the
thin image inherited Debian's `__TAURI_BUNDLE_TYPE_VAR_DEB` marker. Tauri's pinned
`tauri-utils 2.9.3` maps that marker to `Deb`; the AppImage marker is `APP`.
The application currently has no bundle-type API caller, and resource resolution
does not depend on this marker, so the successful startup evidence remains true.
The parent nevertheless required fixing this local packaging defect before
acceptance. The fix is independently accepted: the builder changes exactly three
bytes in the extracted application, preserves the input deb, and records both
ELF hashes and the offset. The gate requires one `APP` marker and verifies the
reverse conversion; Debian inspection independently requires one `DEB` marker.
`test_build_thin_appimage`, `test_appimage_inventory` and
`test_check_release_bundle` passed **57 tests in 0.226 s** in this review round.
The first candidate is superseded. The final build and actual checks above
establish the corrected format metadata after the additional authorized UI work.

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
   cleanup had no blocking review finding; actual candidate and installation
   evidence was still pending at that historical checkpoint. A subsequent
   helper fix was also reviewed:
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
- Earlier tooling-only artifact check and `git diff --check`: passed. Build 15.88 GiB / 19,640
  files, temporary 0.02 GiB / 2,107 files, workspace free 606.04 GiB; only the
  documented 10 GiB build warning remained at that checkpoint. Reviewer-owned
  processes and scratch were cleaned before the actual candidate review reopened.
- Final candidate review cleanup: all reviewer probes exited and were reaped;
  `.scratch/p5f-w3-review-release` was removed. `python3 scripts/check_artifacts.py`
  passed with 5.33 GiB / 12,262 build files, 0.08 GiB / 1,264 temporary files and
  616.29 GiB free. `git diff --check` passed. The thin and packaging owners were
  notified that their retained review resources were no longer needed.

## Remaining evidence boundaries

- Final integrated Linux artifact inspection, isolated APT install/removal and
  bounded native smoke evidence are complete as recorded above. Clean-OS
  dependency resolution and cross-version upgrades were not exercised.
- macOS/Windows package execution and Windows Job Object integration remain
  unverified on this Linux host; workflow definitions do not close audit rows
  9/62. GUI process survival itself is weaker than visible frontend readiness or
  interactive platform parity.
- The user deferred publication and has not configured signing credentials.
  Nothing was signed, pushed, dispatched or published by this reviewer.
