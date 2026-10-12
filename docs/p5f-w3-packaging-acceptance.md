# P5 wave 3: Linux packaging verification

Date: 2026-10-02. Scope: audit row 62, final integrated Linux candidates,
source provenance and actual package-manager verification. **The final deb and
thin AppImage build and strict package checks pass. Actual deb installation,
same-version reinstallation, removal and private user-data retention pass.**
The final build includes the UX and welcome-screen changes plus the corrected
Tauri AppImage bundle marker. Signing, publication and macOS/Windows verification
are excluded by the user's current scope.

## Final candidates and source

Build revision: `f6f6cf9ce481d11314e25c540b5719fd0951a00f`.
The fingerprint covers 2,008 application/tooling inputs:
`2fe41daf9aa27d48230fb3ac970ab119867c49245f2dcdff47017c60c93a6624`.
It stayed unchanged from before notice/API generation until both installers
were produced, and matches the independent reviewer's prebuild checkpoint.

| Candidate in `target/release/bundle/`      |      Bytes | SHA-256                                                            |
| ------------------------------------------ | ---------: | ------------------------------------------------------------------ |
| `deb/FontCarver_0.1.0_amd64.deb`           | 24,235,798 | `e01096f3abd387e5758dc177cfe5a85e4ec5030613d1f3f1c890aac789e429de` |
| `appimage/FontCarver_0.1.0_amd64.AppImage` | 23,683,640 | `3a9029b1aa83d39d78365b18202b39aa5a035f640083dfb4e53151c205793536` |

The [final manifest](acceptance/p5f-w3-packaging/release-manifest.json) records
these exact artifacts and source inputs. Each carries 630 source-package records,
1,126 checksummed dependency notice files, project licenses and Linux host-library
notices with original LGPL texts. The deb carries its required copyright file.
The thin gate verified nine pinned source/license inputs, its runtime and host
linkage, and **zero bundled shared libraries**.

The installed deb ELF has one `DEB` marker at prefix offset 4,743,247. The thin
AppImage has one `APP` value at offset 4,743,271: only the three type bytes change.
Reversing those bytes reproduces the input deb executable's SHA-256. The final
manifest records both main-executable hashes and this transformation.

## Build and actual installation

The build used Ubuntu 24.04.4 x86_64, the shared Cargo target and exclusive Cargo
lock. Isolated thin-image prerequisites were not installed into the host or
copied into the application payload.

```sh
TMPDIR="$PWD/.scratch/p5f-w3-packaging/tmp" PYTHONDONTWRITEBYTECODE=1 \
  flock .scratch/p5f/cargo.lock python3 scripts/build_release.py \
  --bundles deb,appimage \
  --appimage-sysroot "$PWD/.scratch/p5f-w3-thin-appimage/root"
```

[Build log](acceptance/p5f-w3-packaging/build.log): source policy checked 601
Cargo and 277 npm versions; production frontend build passed; cached release
compilation finished in 1m 12s; both strict artifact inspections passed.
Recreating the disposable prerequisites is documented in
[the thin-image acceptance](p5f-w3-thin-appimage-acceptance.md).

```sh
TMPDIR="$PWD/.scratch/p5f-w3-packaging/tmp" PYTHONDONTWRITEBYTECODE=1 \
  python3 scripts/check_deb_install.py \
  target/release/bundle/deb/FontCarver_0.1.0_amd64.deb --version 0.1.0 \
  --output docs/acceptance/p5f-w3-packaging/deb-install.json
```

Actual APT install, same-version reinstall and removal passed, including
hicolor/menu/desktop-file triggers and document/preferences/recovery marker
retention. Both installed-binary probes returned version 0.1.0, exit 0, and the
expected no-display diagnostic, exit 1. All writes occurred in disposable
rootless overlays; the host executable remained absent.

Evidence: [APT report](acceptance/p5f-w3-packaging/deb-install.json),
[full APT log](acceptance/p5f-w3-packaging/deb-install.log),
[installed smoke](acceptance/p5f-w3-packaging/deb-install-smoke.json),
[reinstalled smoke](acceptance/p5f-w3-packaging/deb-reinstall-smoke.json).
These files all describe the final deb digest above.

## Review and superseded evidence

The build helper rejects stale/duplicate candidates and source changes during
preparation or compilation. Review fixes added omitted root pnpm inputs, moved
the source snapshot before preparation, and required actual host notices and
original LGPL text hashes. The bundle-marker regression rejects absent, unknown,
wrong, duplicate or conflicting markers. Focused tools: build helper **7/7**,
bundle verifier **24/24**, isolated installer **6/6**, all passed. No dependencies
or i18n keys were added by this packaging task.

The [first-round archive](acceptance/p5f-w3-packaging/round-1-6e35f70/README.md)
preserves revision `6e35f70` and its original results. That round passed the
checks then available, including actual installation; late review found its
incorrect thin `DEB` marker. Its old manifest is superseded. The final candidates
were rebuilt normally from the frozen final source, not edited in place.
Historical linuxdeploy AppImages also remain unapproved.

[Independent release review](p5f-w3-review-release.md) reproduced the final
artifact inspections, source identity and complete APT evidence. The
[AppImage policy review](p5f-w3-appimage-acceptance.md) verified all notices and
the exact three-byte marker conversion against the actual deb. The
[thin-image runtime review](p5f-w3-thin-appimage-acceptance.md) passed FUSE and
extract-and-run version/no-display probes, complete extraction and 15 seconds
of GUI process survival under isolated Xvfb. All three reviews passed for the
final artifact digests above; GUI survival does not establish interactive parity.

The [synthetic helper log](acceptance/p5f-w3-packaging/synthetic-install.log)
records earlier namespace testing. Failed/interrupted probes were corrected
before the successful runs and are not counted as passes.

## Limits and resources

These APT checks reuse the host's installed dependencies. They do not establish
clean-OS resolution, cross-version upgrades or interactive graphical workflows.
The one-ID namespace produces a harmless APT log-group ownership warning;
nonzero package-manager exits fail verification. No credentials, signatures or
public release were created, and no personal desktop input was automated.

Final build started and finished at 5.33 GiB of shared build artifacts.
Installation namespaces and overlays were automatically removed. After all
independent checks passed, this task removed its entire scratch directory,
including the superseded first-round binary copies, and retained the final
candidate pair and acceptance evidence. Its final resource checkpoint passed:
5.33 GiB / 12,262 build files, 0.08 GiB / 1,266 temporary files, 616.29 GiB free.
Other tasks own the remaining temporary files; the thin-image task separately
records cleanup of its shared build prerequisites after consumer confirmation.
