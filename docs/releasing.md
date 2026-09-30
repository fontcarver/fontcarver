# Building and verifying native packages

FontCarver packages the Tauri desktop application as Linux deb/AppImage, macOS
dmg and Windows msi artifacts. The native package workflow builds reviewable
artifacts; it does not create a GitHub Release, publish downloads, sign Windows
packages, or notarize macOS applications. A green workflow records package
inspection and scripted installation/removal checks; interactive native editing
workflows require separate acceptance.

For package installation and removal, see [install.md](install.md).

## Toolchains and platform prerequisites

Run commands from the repository root. Use the Rust version in
[`rust-toolchain.toml`](../rust-toolchain.toml), Node.js 24 and the pnpm version
pinned by `packageManager` in [`package.json`](../package.json). Python 3.11+ runs
the build helper, verification and resource checks. Install dependencies with
`pnpm install --frozen-lockfile` and retain the committed lockfiles.

Build each package on its native operating system. Linux needs the WebKitGTK
4.1/GTK 3 development packages, a C/C++ build toolchain, `pkg-config`, OpenSSL
development files and Tauri's package tools. The repository CI declares the
package list used for its Linux runner. macOS needs Xcode or Xcode Command Line
Tools. Windows needs Microsoft C++ Build Tools with **Desktop development with
C++**, an MSVC Rust target, WebView2 and the Windows VBScript feature used by WiX
MSI generation. See [Tauri's platform prerequisites](https://v2.tauri.app/start/prerequisites/)
for installation details.

The first package build may download Tauri's platform bundling tools. Build on
the oldest Linux baseline that you intend to validate: a newer build host can
raise the runtime's glibc requirement, including inside an AppImage. See
[Tauri's Linux compatibility guidance](https://v2.tauri.app/distribute/appimage/#limitations).

## Resource budget

Read [CONVENTIONS.md](../CONVENTIONS.md#build-caches-and-temporary-artifacts)
before building. Run this before and after a build batch:

```sh
python3 scripts/check_artifacts.py
```

The build ceiling is 20 GiB/100,000 files, task temporary output is limited to
1 GiB/10,000 files, and local work must retain at least 20 GiB free. CI uses a
documented 5 GiB free-space floor. These are checkpoints, not continuous quotas.
An exceeded budget must be resolved before another build.

Use the shared Cargo target directory and normal space-saving profiles. Keep
task files under `.scratch/<task-id>/`; on Unix give tests an absolute `TMPDIR`
inside that directory. An external task temporary directory must be included in
resource checks with `--task-temp PATH`. Never remove source, user fonts, Git
data, shared dependency stores or another task's outputs as cleanup.

## Native build command

After installing the platform prerequisites and pnpm dependencies, run:

```sh
python3 scripts/build_release.py
```

The helper selects `deb,appimage` on Linux, `dmg` on macOS and `msi` on Windows.
Use `python` in place of `python3` if that is the name of your Python 3.11+
interpreter on Windows. To build only one of the native Linux formats:

```sh
python3 scripts/build_release.py --bundles deb
```

The helper checks resource budgets and version consistency, fetches the locked
Cargo sources, prepares dependency license notices, generates the Rust-owned
TypeScript API and invokes
`pnpm --dir app exec tauri build` with native bundle targets and the locked Cargo
dependency graph. Tauri runs the frontend production build before compiling the
desktop executable. The helper sets the application/frontend paths itself; no
Unix-style environment assignment is required on Windows.

Build output is under `target/release/bundle`, or
`$CARGO_TARGET_DIR/release/bundle` when an alternate Cargo target directory is
already configured. An installed cross target or an arbitrary `--bundles` value
does not make the helper cross-compile another operating system's package.
The native helper rejects `CARGO_BUILD_TARGET` and `build.target` overrides in
Cargo configuration; target-specific output layouts are outside this helper's
supported path convention.

The helper owns a unique `.scratch/p5-release/build-*` directory and removes
that child after the attempt. Keep review logs outside the helper's temporary
directory. It checks generated bundles before reporting success. CI passes
`--min-free-gib 5` for its ephemeral runner; local builds keep the default 20 GiB
floor.

## Inspect packages and retain evidence

The build helper invokes the package verifier. To inspect existing Linux outputs
again without rebuilding:

```sh
python3 scripts/check_release_bundle.py --bundle-dir target/release/bundle --expected deb,appimage
```

Use `--expected dmg` on macOS or `--expected msi` on Windows. Inspection uses
native tools: `dpkg-deb` extraction, AppImage's extraction mode, a read-only DMG
mount or Windows Installer administrative extraction. It does not launch
FontCarver. Missing expected artifacts, missing platform tools or a mismatched
host fail verification.

`release-manifest.json` is written into the bundle directory with each package's
relative path, byte size, SHA-256 digest and payload checks, plus the host
platform and generation time. A failed inspection records `ok: false` and the
error; preserve that report as failure evidence. The default output location can
be changed with `--output PATH`.

The payload checks cover executable format, required notices and their hashes,
and platform metadata. Linux requires a desktop entry and matching icon; DMG
inspection checks the application property list and icon. This verifies package
structure and attribution files, not desktop launch or complete font behavior.

Bundles carry both project licenses, `THIRD_PARTY.md`, an `INSTALL.md` copy of the
user instructions and the generated dependency notice catalog. The catalog
retains dependency versions, declared license expressions and upstream notice
text with checksums. Do not substitute a list of license identifiers for those
notice files.

The generated catalog covers Cargo/npm source packages. It does not inventory
native libraries or bundler runtimes added to an installer, including GTK/WebKit
libraries potentially included by linuxdeploy. The supplemental catalog READMEs
also preserve upstream SDK and historical provenance limitations. Inspect actual
native payloads and complete their redistribution notices before distribution;
a passing source-package audit is not complete artifact license clearance.

Retain the final package report outside regenerable Cargo output before cleaning
a release cache. For a distribution candidate, retain the exact installers and
their manifest together with the source revision and native acceptance results.

## Native CI artifacts

[`release.yml`](../.github/workflows/release.yml) runs on manual dispatch and
pull requests that change its listed packaging inputs. Its current native matrix
is:

| Runner              | Architecture | Artifacts                                   |
| ------------------- | ------------ | ------------------------------------------- |
| Ubuntu 22.04        | x86_64       | deb and AppImage                            |
| macOS 14            | arm64        | dmg                                         |
| Windows Server 2022 | x86_64       | English and Simplified Chinese msi variants |

The workflow builds and inspects packages, then defines installation/removal
checks on disposable runners: APT install/remove on Linux, a DMG app copy into a
temporary Applications directory on macOS, and MSI install/uninstall on Windows.
These steps do not automate native application input. A workflow definition is
not evidence that it has run; retain the actual run result before claiming any
platform passed.

Successful runs upload installers and `release-manifest.json` as
`FontCarver-<platform>-<commit>` artifacts for 14 days. Failure reports/logs use
separate artifacts retained for seven days. Download evidence before it expires.
The workflow has read-only repository permissions and no tag or release
publishing step.

## Acceptance before distribution

Check that the Cargo workspace, npm packages and Tauri configuration use the
intended version. Keep `com.fontcarver.app` stable unless a deliberate application
identity migration is being made. The desktop executable is
`fontcarver-desktop`; the separately built `fontcarver` CLI is not a sidecar.

Run the repository's Rust, frontend and headless end-to-end checks. Native bundle
verification then needs to establish that the expected application executable,
desktop integration, icons and notices are present. Preserve the package digest
and verification report with the exact build revision and operating system.

Before accepting a platform for distribution, record all of the following on
that actual platform:

1. Install the generated artifact in a clean test environment and launch the
   installed application.
2. Open a small redistributable font through the native file dialog, edit a
   glyph, undo/redo, save `.fcf`, close it and reopen it.
3. Export a font and inspect it independently. Check native dialogs, fonts,
   menus and rendering in the platform webview.
4. Verify upgrade and removal behavior, including retention of saved fonts,
   preferences and recovery snapshots.
5. Verify the final signature and, for macOS, notarization, on the exact artifact
   being offered to users.

Package extraction and headless browser tests are useful evidence but cannot
prove these native workflows. Native/global desktop input automation remains
outside the authorized verification scope for the current P5 work. macOS and
Windows require native execution evidence; Linux checks do not accept them.

## Signing and publication prerequisites

Signing and publication are separate work from producing review artifacts.
The current workflow has no release-writing permissions or signing credentials.
Do not add signing credentials to a pull-request job.

For macOS distribution, configure a Developer ID Application certificate,
`APPLE_SIGNING_IDENTITY`, and notarization credentials on a trusted macOS runner.
Tauri supports an App Store Connect API key or Apple ID credentials for
notarization. Retain verification evidence for the signed application and the
notarization ticket. See [Tauri's macOS signing guide](https://v2.tauri.app/distribute/sign/macos/).

Windows distribution needs an Authenticode certificate or approved signing
service, digest/timestamp settings and verification of the signed MSI and
executable. Configure these only in the trusted release environment; no publisher
identity or certificate is supplied by this repository. Tauri exposes the
certificate and custom signing options in its [Windows configuration](https://v2.tauri.app/reference/config/#windowsconfig).

Before publishing, resolve outstanding platform acceptance, signing and bundled
dependency/license review, and prepare accurate release notes with known
limitations. A manual CI build alone does not authorize publication or establish
that these prerequisites have been met.
