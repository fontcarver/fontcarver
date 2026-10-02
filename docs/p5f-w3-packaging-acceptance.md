# P5 wave 3: Linux packaging verification

Date: 2026-10-02. Scope: audit row 62, current-tree candidates, isolated APT
installation, installer/source provenance. Signing and publication are deferred
at the user's request. Final candidate build and installation are pending the
parent's integrated build slot; this record currently accepts tooling only.

## Tooling and review

The build helper defaults to deb on Linux. Explicit AppImage builds use the
reviewed thin builder after a fresh deb. It rejects stale/duplicate candidates,
checks that application/tooling inputs stay unchanged through preparation and
compilation, and records their fingerprint plus Git revision in the manifest.
The package verifier applies the artifact-specific AppImage policy gate.

Focused validation (`TMPDIR` under this task's scratch directory):

```text
python3 -m unittest discover -s scripts -p test_build_release.py -v
Ran 7 tests — OK
python3 -m unittest discover -s scripts -p test_check_release_bundle.py -v
Ran 22 tests — OK
python3 -m unittest discover -s scripts -p test_check_deb_install.py -v
Ran 6 tests — OK
```

Reviewer findings about stale candidates and omitted root pnpm inputs were
fixed with regressions. The source snapshot now precedes license/API generation.

## Isolated installation helper

A synthetic deb with the real icon layout, desktop entry and GTK/WebKit
requirements passed real APT install, same-version reinstall and removal,
including hicolor/menu/desktop-file triggers, exact-version/headless smoke
checks, and private user-data retention. This proves the helper, not the real
application candidate. [Exact log](acceptance/p5f-w3-packaging/synthetic-install.log).

The helper uses rootless user/mount/PID namespaces, read-only host mounts,
private writable overlays and private offline APT configuration. Earlier probes
exposed nested directory permissions and a PID/proc namespace mismatch; those
failed/interrupted runs are not passes. Both defects were corrected before the
successful synthetic run. All those namespace processes and overlays are gone.

## Limits

Runtime dependencies come from the existing host. Clean-OS resolution,
cross-version upgrades and graphical workflows need separate evidence. APT's
log-group ownership warning is harmless in the one-ID namespace; package-manager
failures still reject the run. Native macOS/Windows evidence belongs to their
platform task. No credentials, signing or publication were performed.

## Review round and resources

The independent release reviewer accepted stale-output handling, source
fingerprinting and rootless isolation after the fixes above. A second review
also exercised a real child-process timeout and confirmed that its delayed
marker was never written after cleanup. Final focused rerun: **35 tests passed**.
No dependencies or i18n keys were added by this task.

Before yielding the heavy build slot back to integration, all owned processes,
probe namespaces and `.scratch/p5f-w3-packaging` were removed. The artifact check
passed at 15.88 GiB / 19,640 build files, 0.02 GiB / 2,096 temporary files,
606.04 GiB free; the remaining scratch belongs to other active tasks.
