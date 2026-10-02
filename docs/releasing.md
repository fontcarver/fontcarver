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
package list used for its Ubuntu 24.04 Linux runner. The thin AppImage runtime
uses the pinned SquashFUSE 0.5.0 ABI available on that baseline; Ubuntu 22.04
is not accepted by this build recipe. macOS needs Xcode or Xcode Command Line
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

The helper selects `deb` on Linux, `dmg` on macOS and `msi` on Windows.
Use `python` in place of `python3` if that is the name of your Python 3.11+
interpreter on Windows. To include the maintained thin AppImage on Linux:

```sh
python3 scripts/build_release.py --bundles deb,appimage
```

The helper checks resource budgets and version consistency, fetches the locked
Cargo sources, prepares dependency license notices, generates the Rust-owned
TypeScript API and invokes
`pnpm --dir app exec tauri build` with native bundle targets and the locked Cargo
dependency graph. Tauri runs the frontend production build before compiling the
desktop executable. AppImage requests build a deb first, then run
`scripts/build_thin_appimage.py`; Tauri's default linuxdeploy AppImage recipe is
not used. The thin runtime additionally needs a C compiler, binutils,
`squashfs-tools`, `libsquashfuse-dev` and `libfuse3-dev`. An isolated extracted
prerequisite tree can be supplied with `--appimage-sysroot PATH`; its libraries
are build inputs and must not be copied into the image. The helper sets the
application/frontend paths itself; no
Unix-style environment assignment is required on Windows.

Build output is under `target/release/bundle`, or
`$CARGO_TARGET_DIR/release/bundle` when an alternate Cargo target directory is
already configured. An installed cross target or an arbitrary `--bundles` value
does not make the helper cross-compile another operating system's package.

Reuse the workspace `target` directory. An alternate `CARGO_TARGET_DIR` is
supported for explicitly planned builds, but duplicates development and release
caches and must fit the same combined resource budget. The helper passes that directory to every resource check
automatically, so no extra flag is needed inside the helper; a check you run
yourself alongside it needs `--build-dir "$CARGO_TARGET_DIR"`. Expect the
separate directory to cost extra time and space: the TypeScript API generator
runs through Cargo, so it compiles its own dependency tree in the new directory
in the development profile before the release build starts. A first build into
an empty release directory measured about 6 GiB.
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
platform and generation time. A completed build also records the source revision
and a SHA-256 fingerprint of its non-ignored application/tooling inputs, and
rejects changes to those inputs during preparation or compilation. It rejects
unchanged or duplicate installer candidates left in the reused bundle directory;
move old candidates aside deliberately rather than treating them as new output.
A failed inspection records `ok: false` and the
error; preserve that report as failure evidence. The default output location can
be changed with `--output PATH`.

The payload checks cover executable format, required notices and their hashes,
and platform metadata. Linux requires a desktop entry and matching icon; DMG
inspection checks the application property list and icon. This verifies package
structure and attribution files, not desktop launch or complete font behavior.

A Debian package additionally has to carry the copyright document that Debian
policy 12.5 makes mandatory. Tauri's deb bundler does not write one, so
`bundle.linux.deb.files` installs
[`src-tauri/packaging/linux/copyright`](../src-tauri/packaging/linux/copyright)
at `/usr/share/doc/font-carver/copyright`, and the verifier rejects a package
whose copyright document is missing, empty, outside the installed package's own
directory, or silent about either project license. That destination repeats the
package name Tauri derives from `productName`, so changing the product name
means changing this path too — the verifier is what stops a renamed product from
shipping a document nobody can find.

Bundles carry both project licenses, `THIRD_PARTY.md`, an `INSTALL.md` copy of the
user instructions and the generated dependency notice catalog. The catalog
retains dependency versions, declared license expressions and upstream notice
text with checksums. Do not substitute a list of license identifiers for those
notice files.

The generated catalog covers Cargo/npm source packages. It does not inventory
native libraries or bundler runtimes added to an installer. The supplemental
catalog READMEs also preserve upstream SDK and historical provenance
limitations. Inspect actual native payloads and complete their redistribution
notices before distribution; a passing source-package audit is not complete
artifact license clearance.

### AppImage recipe and payload gate

The maintained recipe builds a thin image from the just-built deb. Its runtime
is compiled from pinned permissively licensed sources and dynamically loads
FUSE 3 and SquashFUSE from the host. GTK, WebKitGTK and their shared-library
stack also remain host dependencies. The image carries a build receipt and
runtime notices. Package verification checks that receipt against the actual
runtime and extracted payload; an unreviewed runtime or bundled shared object
fails the AppImage policy gate.

The historical Tauri/linuxdeploy image used a different recipe: 177 bundled
shared objects from 126 Debian packages, including 73 packages whose copyright
files mention copyleft, with only 10 package copyright documents shipped. Its
AppImage runtime also statically linked LGPL libfuse. That image remains an
unapproved evaluation artifact. Adding notices alone does not make that recipe
meet the repository's shipped-dependency policy. Historical evidence is in
[the payload inventory](acceptance/p5f-release/appimage-bundled-licenses.txt).
For diagnostic inventory of an already extracted image, use
`scripts/appimage_inventory.py`; diagnostic output is not release clearance.

Retain the final package report outside regenerable Cargo output before cleaning
a release cache. For a distribution candidate, retain the exact installers and
their manifest together with the source revision and native acceptance results.

## Rootless Linux package-manager verification

On a Linux host with unprivileged user namespaces, overlayfs and bubblewrap,
verify a candidate without installing it into the personal system:

```sh
python3 scripts/check_deb_install.py \
  target/release/bundle/deb/FontCarver_0.1.0_amd64.deb --version 0.1.0 \
  --output .scratch/p5f-w3-packaging/deb-install.json
```

The helper makes the host root read-only, mounts task-owned writable overlays,
uses private APT configuration with downloads and networking disabled, and runs
real APT install, same-version reinstall and removal. It checks package state,
installed resources, the binary's exact version and expected no-display refusal,
and retention of private document/preferences/recovery markers. Its namespace
and overlays are removed even on failure. It never sends native desktop input.

This reuses the host's installed dependency set. It does not prove clean-OS
dependency resolution, a cross-version upgrade, graphical workflows or signing.
A one-ID user namespace may produce an APT log-group ownership warning without
affecting the package operations; nonzero package-manager exits fail the check.

## Native CI artifacts

[`release.yml`](../.github/workflows/release.yml) runs on manual dispatch and
pull requests that change its listed packaging inputs. Its current native matrix
is:

| Runner              | Architecture | Artifacts                                   |
| ------------------- | ------------ | ------------------------------------------- |
| Ubuntu 24.04        | x86_64       | deb and AppImage                            |
| macOS 14            | arm64        | dmg                                         |
| Windows Server 2022 | x86_64       | English and Simplified Chinese msi variants |

A manual dispatch takes an optional `bundles` input: any comma-separated subset
of `deb`, `appimage`, `dmg` and `msi`, case-insensitive, empty for every
platform's native formats. A short `plan` job resolves that input into the
matrix, so an unrecognised format name fails in seconds instead of after a
90-minute build, and a runner whose formats were all excluded never starts.
Each runner also restores the Cargo registry (sources only — a release `target`
directory is far too large to cache), so a transient crates.io failure does not
decide a three-platform packaging run.

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
intended version. Keep the application identifier stable unless a deliberate
identity migration is being made. The desktop executable is
`fontcarver-desktop`; the separately built `fontcarver` CLI is not a sidecar.

Two naming facts are established only on Linux, where a package has actually
been built and read: the executable installs as `/usr/bin/fontcarver-desktop`,
and the Debian package name is `font-carver`, which Tauri derives from
`productName`. The bundle verifier also expects
`Contents/MacOS/fontcarver-desktop` inside the macOS app bundle and
`fontcarver-desktop.exe` in the MSI. Those two are assumptions about how the
bundler names a binary on platforms no build has run on; the first macOS and
Windows runs either confirm them or fail the verifier, which is the intended
outcome either way.

The identifier is `com.fontcarver.desktop`. It was `com.fontcarver.app` until the
Tauri CLI's warning on every build — an identifier ending in `.app` collides with
the macOS application bundle extension — was acted on during the P5 completion
wave, while nothing had been released under the old value. The Windows upgrade
code is pinned explicitly in `tauri.conf.json`, so it did not move with the
identifier, and the configuration directory is named `fontcarver` independently of
both. The first Linux packages were built under the old identifier; it appears in
macOS and Windows bundle metadata rather than in a deb's control fields, so no
Linux evidence recorded for those packages depends on it.

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
