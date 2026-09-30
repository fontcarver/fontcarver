# P5 native release packaging checkpoint

Date: 2026-09-30. Native build definitions, resource configuration, user/developer
instructions, and installer inspection are implemented. This checkpoint does not
claim a FontCarver installer build or an accepted native editing workflow.

## Implemented

- `scripts/build_release.py` chooses native deb/AppImage, dmg, or msi targets,
  checks Cargo/frontend/Tauri versions against MSI version limits, fetches locked
  Cargo sources, prepares dependency notices, regenerates TypeScript APIs, builds
  the Tauri frontend/application and inspects installer contents. It uses portable
  environment variables rather than Unix-only npm command assignments.
- Tauri bundle metadata includes the stable application identity, graphics
  category, both project licenses, third-party attribution and installation
  instructions. macOS declares 11.0 as its build deployment target. Windows
  retains Tauri's current derived upgrade GUID explicitly, disables downgrades,
  supplies en-US/zh-CN installer variants and bootstraps WebView2 when needed.
- `.github/workflows/release.yml` defines Ubuntu 22.04 x86_64, macOS 14 arm64 and
  Windows Server 2022 x86_64 builds. It specifies package inspection and native
  installation/removal on disposable runners, then uploads installers and JSON
  evidence. It has read-only repository permission and no publication, tag,
  signing or notarization step.
- `scripts/check_release_bundle.py` extracts native packages, verifies executable
  format/location, platform metadata/icons, notices and notice checksums, and
  emits installer SHA-256 manifests. It rejects missing resources, wrong hosts,
  unsafe notice paths and FontForge implementation payloads. Extraction scratch
  is removed; a mounted DMG is detached on validation failure.
- [Installation instructions](install.md) and [developer release instructions](releasing.md)
  cover build/install/remove commands, retained user data, artifact evidence,
  signing requirements and remaining platform acceptance.

The automation owner requested the coordinated CSP addition `script-src 'self'
blob:; worker-src 'self'` for trusted macro module imports. Native macro execution
is not established by configuration validation.

## Observed verification

```text
python3 -m unittest discover -s scripts -p 'test_*release*.py'
................................
Ran 32 tests in 2.974s
OK
```

The 17 bundle-verifier cases include an actual synthetic Debian package built
with `dpkg-deb`, extracted and inspected successfully. The payload is deliberately
synthetic; this exercises packaging validation without pretending to launch the
FontCarver application. Other cases cover missing expected artifacts, modified
notices, nested resource layout, escaping paths, extraction cleanup and DMG
detachment after errors. Build-helper cases reject version mismatch, unsupported
platforms and invalid MSI versions. Regression cases reject Cargo target
overrides that could otherwise make a fresh build validate stale installers in
the native output directory.

The live license checker passed after the shared Cargo lockfile was refreshed:
601 Cargo and 277 npm versions satisfy the repository's recorded license policy.
Generated notices cover 601 Cargo and 29 production npm versions, with 1,126
notice checksums verified (about 6.17 MB). Published packages that omit their
license documents use exact-version supplemental catalogs with recorded source
URLs and SHA-256 hashes. No new dependency or license-policy exception was added.

This source-package audit does not clear every native redistribution obligation.
The catalogs preserve upstream objc2 Apple SDK caveats, a missing independent
WebView2 SDK loader redistribution notice, and difflib/WinAPI historical source
provenance limits. Actual AppImage GTK/WebKit libraries and bundler runtimes have
not been inventoried. Review those native payloads and notices before public
distribution.

The checked-in bundle configuration validates against the installed Tauri CLI's
configuration schema. The workflow parses as YAML, has three native matrix rows,
and grants only `contents: read`. Documentation links, shell examples and
Prettier checks passed. Generated notice and extraction scratch was removed after
verification; final resource totals are recorded below.

```text
python3 scripts/check_artifacts.py
build: 6.19 GiB, 13,213 files
temporary: 0.21 GiB, 6,157 files
workspace free: 668.69 GiB
PASS
```

These totals include other concurrent tasks. This release task retained source,
the acceptance record and the small versioned notice catalog; its generated
notice bundles, metadata snapshots and extraction directories were removed.

## Unverified boundaries

The user subsequently asked to finish work already underway and stop. Since the
parent had reserved broad release compilation until combined Rust/frontend work
was stable, no FontCarver deb/AppImage build had started; it remains unattempted
at this checkpoint. No systemwide installation or native/global desktop input
automation occurred on the user's host.

macOS/Windows workflow definitions have not run here. Even successful package
extraction or scripted installation would not establish native dialogs, keyboard
parity, open/edit/undo/save/reopen/export, upgrade behavior or signed distribution.
Audit rows 9 and 62 therefore remain pending native evidence. The P5 release item
remains open for real installer build/inspection and actual platform acceptance.

## Update — 2026-09-30, p5f-n: the packages were built

The "unattempted" boundary above is superseded. The Linux deb and AppImage of the
current code have been built, inspected and executed as far as a host without
`sudo` or a usable display allows. The full record, with digests, payload
listings, the dependency analysis and the workflow review, is
[p5f-n-release-acceptance](p5f-n-release-acceptance.md); its small text evidence
is under [acceptance/p5f-release](acceptance/p5f-release/).

Headline results: `font-carver` 0.1.0 amd64, a 23.3 MB deb and a 95.8 MB
AppImage, both passing `check_release_bundle.py` and `desktop-file-validate`,
with all 15 of the executable's direct shared-library dependencies covered by the
two packages the deb declares. Two defects were found in the artifacts: the deb
shipped no `/usr/share/doc/font-carver/copyright`, which Debian policy requires —
the verifier now rejects a package without it, and the document is wired into the
bundle configuration, but the rebuild that would carry it was blocked by other
agents' in-progress kernel edits, so it awaits the wave's final build; and the
AppImage bundles
177 shared objects from 126 Debian packages, 73 of which name a copyleft license,
with notices for only 10 inside the image, so the AppImage is not redistributable
yet. Two further defects are outside that task's ownership: the desktop binary has
no `--version`/`--help` and panics rather than diagnosing a missing display, which
leaves no way for any platform's CI to check that an installed binary runs; and the
`com.fontcarver.app` identifier collided with the macOS bundle extension, which
another task in the wave has since fixed by moving it to `com.fontcarver.desktop`.

The macOS and Windows statements above still stand: no dmg or msi of this code
exists, and rows 9 and 62 remain **Partial** — real Linux application-package
evidence, macOS and Windows execution pending on the platform runners.
