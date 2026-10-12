# p5f-n: Packaging and platform acceptance

Date: 2026-09-30. Task: [`p5f-n-release`](superpowers/plans/tasks/p5f-n-release.md).
Rows: feature-audit **9** (cross-platform parity, packaging half) and **62**
(test install on all three platforms), plus the P5 release block.

This is the first record in the project backed by **real FontCarver installers**.
Every earlier release record verified configuration, helper logic and synthetic
payloads; none had built a package. The Linux deb and AppImage of the current
code now exist, have been read apart, and have been executed as far as a host
without `sudo`, and without driving its desktop, allows.

## Scope and environment

Host: Ubuntu 24.04.4 LTS, x86_64, 20 cores, 62 GiB RAM. No `sudo`, no
`appimagetool`, no `xvfb-run`, no system `patchelf`. `dpkg-deb`,
`desktop-file-validate`, `pkg-config`, `glib-compile-schemas`, `/dev/fuse` and
Tauri's cached linuxdeploy tools are present; linuxdeploy carries its own
`patchelf` and `strip`, which is why AppImage bundling worked without `sudo`.

The host was expected to have no display. It has one: a live Wayland session,
which GTK reaches even when `DISPLAY` and `WAYLAND_DISPLAY` are unset. Driving
that desktop is outside the authorized scope, so every execution check here
forces a GTK backend that cannot connect. One early probe did reach the real
session before this was understood; it is recorded below rather than omitted.

Seven other agents were editing the same checkout throughout, so the packaged
code is a moving snapshot of the P5 completion wave, not a fixed revision. The
build ran against `449e753` plus the wave's uncommitted working tree.

The release build used a Cargo target directory outside the workspace,
`/home/chen/.cache/fontcarver-release-target`, so the shared development cache
was never touched, and every local resource check named it with `--build-dir`.
The shared cargo flock was held only for the `scripts/build_release.py`
invocations themselves; the review, inspection and documentation work in this
record ran without it.

## The packages

| Artifact                          |      Bytes | SHA-256 (first 32)                 |
| --------------------------------- | ---------: | ---------------------------------- |
| `FontCarver_0.1.0_amd64.deb`      | 23,314,810 | `3d349aa079e02e9f490fc111a2a75f04` |
| `FontCarver_0.1.0_amd64.AppImage` | 95,799,800 | `13e9560e0c466b0b56264f037538bb36` |

Full digests, control metadata, payload listings and the dependency analysis are
in [`acceptance/p5f-release/`](acceptance/p5f-release/). Installers themselves
are not committed; they live in the release target directory only.

The committed
[`release-manifest.json`](acceptance/p5f-release/release-manifest.json) is the
record, and it is the passing run from the build itself (11:08:39Z). The copy
left in the release target directory no longer matches it: later verifier reruns
overwrite that file in place, and the run that added the Debian copyright check
rewrote it to `ok: false` with
`missing Debian copyright document at usr/share/doc/font-carver/copyright`. Both
states are true of what they describe — the package passed every check that
existed when it was built, and it fails the check added afterwards — which is why
the evidence copy is kept under `docs/` rather than read back out of a directory
any rerun can rewrite.

Command, with the shared cargo flock held for the invocation alone:

```sh
CARGO_TARGET_DIR=/home/chen/.cache/fontcarver-release-target \
  flock .scratch/p5f/cargo.lock python3 scripts/build_release.py --bundles deb,appimage
```

```text
Finished `release` profile [optimized] target(s) in 3m 36s
   Bundling FontCarver_0.1.0_amd64.deb
   Bundling FontCarver_0.1.0_amd64.AppImage
   Finished 2 bundles
PASS: /home/chen/.cache/fontcarver-release-target/release/bundle/release-manifest.json
Verified FontCarver 0.1.0 native installers
```

The helper needed no change for the alternate target directory: it makes
`CARGO_TARGET_DIR` absolute, derives the bundle directory from it, and passes it
to every resource check through the environment. The first build held the lock
from 13:02:52 to 13:08:41 — 5 minutes 49 seconds for the whole pipeline,
including the license audit, the API generation and the package inspection.

**The packages above predate the Debian copyright fix in finding 1.** They are
the artifacts every measurement in this record was taken from. The rebuild that
would carry the copyright document has not completed: seven other agents were
editing the same checkout, and every attempt failed on their in-progress code.
First in Cargo — `` `?` couldn't convert the error to `fc_doc::Error` ``, then a
missing `LayerKind::extra` field, then removed `BracketRule` fields — and once
the kernel crates compiled again, in the frontend, where `tsc -b` reported
roughly twenty type errors across another task's export, plugin, palette and
history tests. Each failed attempt held the shared cargo lock for seconds and
produced no artifacts, so nothing stale was verified.

That second failure mode is worth recording on its own, because it is a property
of the packaging pipeline rather than an accident: `tauri build` runs
`pnpm --dir app build`, which is `tsc -b && vite build`, and `tsc -b` covers the
app's test files as well as its sources. One broken test file anywhere in the
frontend therefore blocks a release build. It is defensible — nobody wants to
package code that does not type-check — but it makes packaging the single most
fragile step to attempt in a shared checkout, and it is worth knowing before
scheduling a release build alongside other work.

The same pipeline also regenerates `app/src/api/generated/` on every run, since
it invokes the TypeScript API generator through Cargo. That directory is
gitignored and every agent regenerates it, so a release build cannot dirty the
commit — but while the kernel crates are mid-edit the regenerated output can
reference types that were not emitted, which is its own way of breaking the
frontend type-check for whoever runs next. A release build is best scheduled
when the tree is quiet.

What that leaves proven and unproven is worth being exact about. Proven: the
check that rejects a deb without its copyright document, against the real
package (it fails, with the expected message) and against four fixtures; the
DEP-5 document itself; and that `bundle.linux.deb.files` is the supported Tauri
mechanism, present in the 2.11.4 configuration schema, with the configuration
validating against that schema.

Also proven, by a dpkg round trip on the real payload: the built package was
unpacked with `dpkg-deb -x`/`-e`, the DEP-5 document placed at the policy path,
the package rebuilt with `dpkg-deb --build` and re-verified. It passes, with
`Package: font-carver`, the document recorded at
`usr/share/doc/font-carver/copyright` and 1,142 payload files — one more than the
package Tauri produced. The same verifier rejects Tauri's own package. Details in
[`acceptance/p5f-release/deb-copyright-roundtrip.txt`](acceptance/p5f-release/deb-copyright-roundtrip.txt).

That leaves exactly one unproven step: whether a Tauri build places the file at
that path. The wave's final rebuild settles it, and if the path is wrong the
verifier fails the package rather than shipping it.

Six rebuild attempts were made in all, over about twenty minutes, before the
attempt was abandoned: three on the protocol's retry schedule and three on a
longer one, the last of which was stopped by the shared build budget crossing its
ceiling rather than by a compile error. See the resource section at the end.

## What the packages turned out to be

The Debian package is `font-carver` — Tauri derives that name from
`productName`, and it is the name `sudo apt remove` needs, so
[the installation page](install.md) now states it. It declares
`Depends: libwebkit2gtk-4.1-0, libgtk-3-0`, which is a short list for a
GUI application, so it was checked rather than assumed: the executable has 15
direct `DT_NEEDED` libraries besides the loader, every one of them is provided
by a package inside the recursive dependency closure of those two declarations
(660 packages), and nothing is uncovered. On Ubuntu 24.04 the `libgtk-3-0`
name is satisfied through `libgtk-3-0t64`, which provides it.

`desktop-file-validate` reports no findings for the desktop entry in either
package, and the entry's `Exec`, `Icon` and `StartupWMClass` all name
`fontcarver-desktop`, matching the installed executable and the three hicolor
icon sizes.

## Findings

### 1. The Debian package had no copyright document (enforced and configured; rebuild pending)

Debian policy 12.5 requires `/usr/share/doc/<package>/copyright`, and Tauri's
deb bundler writes none. The first build shipped both project licenses under
`/usr/lib/FontCarver/licenses/` but nothing at the path Debian tools and users
look in, which is a `lintian` error and a distribution blocker.

Fixed test-first. `scripts/check_release_bundle.py` gained a `debian_copyright`
check that rejects a deb whose copyright document is missing, empty, outside the
installed package's own `usr/share/doc/<Package>/` directory, or silent about
either project license; four tests cover those cases and one asserts that an
AppImage is not held to the Debian requirement. The check was then run against
the real first-build deb and failed with
`missing Debian copyright document at usr/share/doc/font-carver/copyright`,
which is the defect reproduced against the actual artifact rather than a
fixture. [`src-tauri/packaging/linux/copyright`](../src-tauri/packaging/linux/copyright)
is a DEP-5 document naming both licenses and pointing at the bundled dependency
attribution, and `bundle.linux.deb.files` is configured to install it — see the
note under "The packages" for what the blocked rebuild leaves unconfirmed.

Because the destination repeats the derived package name, the verifier compares
it against the deb's own `Package` field: renaming the product now fails
verification instead of quietly shipping a document at an unreachable path. The
check accepts `Expat` as well as `MIT`, since that is Debian's own short name for
the same license and a DEP-5 document may legitimately use it.

Review also corrected the document itself. `Upstream-Contact` held a URL where
DEP-5 wants a name and address, and there was no stanza acknowledging that
`usr/bin/fontcarver-desktop` statically links its whole Rust dependency graph and
embeds the built frontend — so the document described only FontCarver's own
source while the file it covers carries several hundred other copyrights. It now
has a second `Files` stanza for the executable that names those holders and
points at the inventory shipped beside it in `/usr/lib/FontCarver/licenses/`,
with the explanation in a `Comment` field rather than in a license body, so the
two `MIT or Apache-2.0` stanzas do not disagree about what that short name means.

### 2. The AppImage payload is an unmet licensing obligation (recorded, not fixable here)

`docs/releasing.md` had recorded that native libraries added by the bundler were
never inventoried. They now have been, because there is finally an artifact to
inventory. The AppImage contains **177 bundled shared objects from 126 Debian
packages**; **73** of those packages name a copyleft license in their own Debian
copyright file; and the image ships copyright documents for **10** of them, the
ones linuxdeploy's GTK plugin happens to copy. The generated dependency catalog
inside the image covers none of them, because it inventories Cargo and npm
source packages only.

The full inventory, with each object, its Debian package and that package's
license identifiers, is
[`acceptance/p5f-release/appimage-bundled-licenses.txt`](acceptance/p5f-release/appimage-bundled-licenses.txt).
A copyleft identifier in a Debian copyright file does not prove the linked
object is copyleft — those documents cover whole source packages — so this is a
list of packages needing a per-file ruling, not a list of violations. What it
does establish is that the AppImage cannot be published on the strength of the
source-package audit alone, and that the deb, which links these libraries from
the system instead of copying them, does not carry the same obligation.
[`THIRD_PARTY.md` §3.4](../THIRD_PARTY.md) now says so where the surveys are
described, because "no GPL crate appears in the graph" is easy to read as a
statement about the shipped artifact.

### 3. The desktop binary cannot be smoke-tested (outside this task's ownership)

`fontcarver-desktop` has no `--version` and no `--help`: every flag reaches the
Tauri runtime, which starts a window. On a host with a display that means a real
application launch; on a host without one it means the process reaches
`gtk::init`, fails, and aborts with an unhandled panic
(`Failed to initialize gtk backend!`, exit 101) that names crate internals
rather than telling the user a display is required.

Both halves matter for packaging. There is no way for any platform's CI to check
that an installed binary works beyond "the file is executable", and a user whose
session is broken gets a panic. The fix belongs in
`src-tauri/src/main.rs` or `src-tauri/src/lib.rs` — neither is in this task's
ownership list — and is: handle `--version`/`--help` from `std::env::args()`
before `fontcarver_lib::run()` initialises anything, print the version and
exit 0; and convert the GTK initialisation failure into a diagnostic naming
`DISPLAY`/`WAYLAND_DISPLAY` with a non-zero exit instead of a panic. With
`--version` in place, all three platform runners can assert that the installed
executable runs.

### 4. The bundle identifier collided with the macOS bundle extension (reported, since fixed by another task)

The Tauri CLI warned on every build that `com.fontcarver.app` ends in `.app`,
which conflicts with the macOS application bundle extension. `identifier` is a
top-level key and this task owns the `bundle` section only, so it was reported
rather than changed, with the argument that nothing had been released under it,
the Windows upgrade code is pinned explicitly so it would not move, and the
configuration directory is named `fontcarver` independently of both.

Another task in this wave has since applied it: the identifier is now
`com.fontcarver.desktop`. The packages measured in this record were built before
that change, under the old value. Nothing recorded here depends on it — Tauri puts
the identifier in macOS and Windows bundle metadata, not in a Debian control
file — so the Linux evidence stands as taken. The first macOS or Windows build is
where the new identifier first reaches an artifact.

### 5. Workflow faults found by review

Both workflows were read line by line for faults that only a macOS or Windows
runner would expose. What reasoning could settle is fixed in
[`release.yml`](../.github/workflows/release.yml) and
[`ci.yml`](../.github/workflows/ci.yml):

- The macOS step selected its disk image with `find ... -print -quit`, which
  silently takes the first of however many are there. It needed the count check
  the Linux step already had — but the Linux step gets that check from `mapfile`,
  a bash 4 builtin, and GitHub's macOS runners still provide bash 3.2 for
  `shell: bash`. Copying the Linux idiom across would have broken the first
  `dmg` run. Both steps now collect `find` output with a `while read` loop that
  works on bash 3.2, and both assert exactly one artifact.
- The macOS step hard-coded `FontCarver.app` and `Contents/MacOS/fontcarver-desktop`.
  It now discovers the single `.app` on the mounted image and reads
  `CFBundleExecutable` out of the copied `Info.plist` to decide which binary to
  test, so a bundler naming difference produces a clear failure rather than a
  confusing one. `hdiutil detach` is retried before falling back to `-force`,
  since a busy mount point is a routine CI flake.
- The Windows step would have reported success on an installer that installed
  nothing verifiable. It now requires at least one MSI, checks that
  `%ProgramFiles%\FontCarver\fontcarver-desktop.exe` exists after the install
  and is gone after the uninstall, lists what the installer actually left behind
  when that file is missing, and keeps a verbose log for the uninstall as well
  as the install.
- `workflow_dispatch` had no inputs. It now takes a `bundles` list, resolved by
  a short `plan` job into the matrix, so an unknown format name fails in seconds
  and a runner with nothing left to build never starts. The resolver was tested
  directly against the workflow file for the empty, single-format,
  multi-platform, mixed-case and unknown-name cases.
- Neither Linux nor Windows checked that the package actually runs. The Linux
  job now launches the installed executable, and a new step launches the
  AppImage, each against a GTK backend it cannot reach: a complete runtime gets
  as far as the display check and exits, while a missing library fails earlier
  and a hang is caught by `timeout`. This is the only executable check available
  to a runner with no display, and it was validated locally against both real
  artifacts before being written into the workflow.
- The Linux job also asserts the copyright document is installed and that the
  executable is gone after `apt-get remove`.
- `release.yml` fetched roughly 600 crates on every runner with no cache. It now
  restores the Cargo registry the way `ci.yml` does, so a transient crates.io
  failure does not decide a three-platform run. Its `pull_request` path filter
  gained `scripts/check_artifacts.py`, `pnpm-workspace.yaml`, `docs/releasing.md`
  and the frontend inputs that the production build reads.
- `ci.yml` ran Cargo without `--locked` while the release build requires it, so
  CI could stay green over a lockfile that makes packaging fail. All four Cargo
  invocations now pass `--locked`. A `concurrency` group was added, since `push`
  and `pull_request` currently run the same commit twice; runs on the default
  branch are still allowed to finish.
- The e2e job's robustness was checked rather than changed: it already caches
  `~/.cache/ms-playwright`, already passes `--min-free-gib 5` to every resource
  check, has a 20-minute step timeout inside a 45-minute job, and uploads its
  report on failure.

Review then found four more faults in this task's own workflow changes, all now
fixed:

- The AppImage smoke step ran under `set -uo pipefail` without `-e`, which makes
  a bare `test "${#images[@]}" -eq 1` decide nothing: two AppImages would have
  been reduced to the first without a word. It now uses `set -euo pipefail` with
  `status=0; output="$(…)" || status=$?`, so the count check is fatal and the
  expected non-zero exit is still captured.
- Both launch checks piped into `head -5`, which can fail the step on `SIGPIPE`
  under `pipefail` for no reason other than output length. They use
  `head -5 <<<"$output"` instead.
- The Windows step asserted `fontcarver-desktop.exe` while `releasing.md` calls
  that name unverified on Windows. It now discovers the executables the installer
  placed, prints them, and only then compares against the name
  `check_release_bundle.py` requires — so the first Windows run reports what the
  bundler actually did instead of just failing. Its `Get-ChildItem` calls carry
  `-ErrorAction SilentlyContinue`, since pwsh runs with `$ErrorActionPreference`
  set to `Stop` in Actions and a missing directory would otherwise throw a raw
  error in place of the intended "no MSI installer was produced".
- The planner had an unreachable "no runner builds the requested formats" branch,
  since a validated non-empty request always matches a row; it is gone. The macOS
  and Windows steps now carry the same `contains(matrix.bundles, …)` guard as the
  Linux ones, so narrowing a dispatch cannot leave a step running against a
  format that was not built.

The Windows step is the one piece checked only structurally: `pwsh` is not
installed here, so its brackets and quoting were verified but the script was never
parsed by PowerShell.

What only a run can prove: whether WiX MSI generation finds its prerequisites on
`windows-2022`; whether Tauri's DMG step completes without a GUI session (it
passes `--ci`, and GitHub sets `CI`, which is what makes `bundle_dmg.sh` skip its
AppleScript, but that path is untested here); and whether the bundler names the
macOS and Windows binaries `fontcarver-desktop`/`fontcarver-desktop.exe` as
`check_release_bundle.py` expects. The macOS and Windows bundler templates are
not present in the Linux CLI binary, so that last point could not be settled by
reading either.

## Commands and results

```text
python3 -m unittest discover -s scripts -p "test_*release*.py"
Ran 37 tests in 0.218s
OK

env PATH=/nonexistent python3 -m unittest discover -s scripts -p "test_*release*.py"
Ran 37 tests in 0.119s
OK (skipped=1)
```

Thirty-two of those tests existed before; five are the new Debian copyright
cases. The suite includes a real `dpkg-deb --build` round trip on a synthetic
payload, and that case is the only one needing `dpkg-deb`.

The second run is there because review caught a real defect in the first version
of these tests: inserting the new cases directly after the
`@unittest.skipUnless(shutil.which('dpkg-deb'), …)` decorator silently moved the
guard onto the first inserted test, leaving the round-trip case — the one that
actually shells out to `dpkg-deb` — unguarded. On a runner without `dpkg-deb`
the suite would have errored instead of skipping, and `release.yml` runs it on
macOS and Windows before anything is built, so both non-Linux jobs would have
failed at that step. The guard now lives inside the body of the test it protects,
where a later insertion cannot move it, and the run with `PATH=/nonexistent` is
what confirms the behaviour: one skip, no errors.

```text
python3 scripts/check_release_bundle.py \
  --bundle-dir /home/chen/.cache/fontcarver-release-target/release/bundle \
  --expected deb,appimage
PASS: .../release-manifest.json
```

That run is the one inside the build pipeline, and it passed the checks that
existed at the time. Run again with the new Debian copyright check, the _same_
artifact fails:

```text
missing Debian copyright document at usr/share/doc/font-carver/copyright
FAIL
```

which is the defect this record reports, reproduced against the built package
rather than a fixture.

```text
flock .scratch/p5f/cargo.lock python3 scripts/check_release_licenses.py
PASS: source-package policy verified for 601 Cargo and 277 npm versions;
notice documents checked for 601 Cargo and 29 production npm versions
```

Package inspection, all run against the real artifacts:

```text
dpkg-deb --info  -> Package: font-carver, Version: 0.1.0, Architecture: amd64,
                    Installed-Size: 71854, Section: graphics, Priority: optional,
                    Depends: libwebkit2gtk-4.1-0, libgtk-3-0
dpkg-deb -c      -> 1,141 payload files; 1,126 of them dependency notices
dpkg-deb -x      -> layout in acceptance/p5f-release/deb-contents.txt
desktop-file-validate -> no findings, on the deb entry and on both copies of
                         the AppImage entry in its extracted payload
--appimage-extract    -> 1,475 regular files, 1,510 counting symlinks (which is
                         what the manifest's file_count reports), 688 directories,
                         307 MiB expanded
objdump -p + dpkg -S + apt-cache depends --recurse
                 -> 15 direct DT_NEEDED libraries, all covered, none uncovered
release-manifest.json -> 630 dependency packages, 1,126 notice files, all
                         checksums verified, for both artifacts
```

Configuration and workflow checks:

```text
jsonschema against @tauri-apps/cli 2.11.4 config.schema.json -> valid
Cargo workspace / app/package.json / tauri.conf.json versions -> 0.1.0, 0.1.0, 0.1.0
licenses (workspace / app / root)                             -> MIT OR Apache-2.0
icons declared for all three platforms                        -> all files present
bundle resources                                              -> all sources present
yaml.safe_load on ci.yml, release.yml, site.yml               -> parse
bash -n on every `shell: bash` step of release.yml           -> syntax OK
AppImage smoke step body, run verbatim against the artifact   -> status 101, passes
same body with two AppImages present                          -> exits 1, as intended
matrix resolver, 6 accepted and 2 rejected inputs            -> all as expected
prettier --check on every Markdown file authored or edited here,
  except THIRD_PARTY.md                                       -> clean
```

`THIRD_PARTY.md` is excluded deliberately, not overlooked. Prettier rewrites its
pre-existing fixture and dependency tables — it does so to the version committed
at `HEAD`, before any change here — and the protocol forbids reformatting a
shared file. It was checked once, the reformatting was reverted, and the change
that remains is a 19-line addition with no other line touched.

## What could not be executed on this host

- **No systemwide installation.** `apt install ./FontCarver_0.1.0_amd64.deb`
  needs `sudo`. The payload was verified by `dpkg-deb -x` into scratch instead,
  which establishes the file layout, permissions, metadata and dependency
  declarations but not APT's own resolution, the maintainer-script path, or
  removal behaviour. The CI workflow defines those steps on a disposable runner.
- **No macOS or Windows anything.** No dmg or msi of this code exists. Their
  workflow definitions are reviewed and, where reasoning could settle a fault,
  corrected; execution remains **pending on the platform runners**. Nothing in
  this record closes the macOS or Windows half of rows 9 and 62.
- **No `appimagetool`, no `xvfb-run`, no `sudo`.** AppImage bundling worked
  anyway, through Tauri's cached linuxdeploy and its bundled `patchelf`.
- **No interactive application workflow.** Opening a font, editing a glyph,
  undo/redo, `.fcf` save and reopen, export, native dialogs, menus, upgrade and
  removal behaviour are all untested for a _packaged_ build. Native and global
  desktop input automation remains prohibited, and package inspection cannot
  substitute for it.

### A launch that should not have happened

While probing for a display-free smoke test, `fontcarver-desktop --version` and
then the bare binary were run with `DISPLAY` and `WAYLAND_DISPLAY` unset, on the
assumption that this host had no display. It does: GTK ignored the unset
variables and connected to the default `wayland-0` socket in `XDG_RUNTIME_DIR`,
so the application really started — the process tree showed WebKit's
`WebsiteDataStore`, the GTK main loop, FontCarver's autosave and event threads
and its tokio workers — and a window existed for a few seconds before the
process was killed. Four earlier flag probes had blocked for up to 25 seconds
each for the same reason.

The four flag probes ran in the same environment as the bare launch, so they
reached the same live session and each held a window for up to the 25 seconds
before `timeout` killed it. That is an inference from identical conditions rather
than an observation: their process trees were not examined at the time, because
the probes were still believed to be failing on a missing display. Five launches
in total, then, not one.

No input was synthesized, nothing was clicked or typed and no document was
opened; the processes were killed and none remain. Every directory the
application keeps state in was checked afterwards, not only the configuration
root: `~/.config/fontcarver/autosave` is untouched since 2026-09-26, so no
recovery snapshot was written; `prefs.json` is untouched since the same date;
`~/.local/share/fontcarver` and `~/.cache/fontcarver` do not exist at all; and
nothing anywhere under those paths has an mtime inside the launch window. The one
file that did change, `recent.json`, changed later and lists only other tasks'
scratch paths. The launches left no data behind. It was still a real desktop launch on the user's session rather than the
intended display-less check, and the protocol's prohibition covers exactly this
territory. Every later execution check forced GTK at a backend it cannot reach
(`GDK_BACKEND=broadway`, and `GDK_BACKEND=x11` with `DISPLAY=:99`), which fails
before any window is created.

The accident is also the strongest evidence in this record, and it is recorded
rather than dropped for that reason: the packaged Debian binary does start,
initialise WebKitGTK, and bring up the FontCarver kernel's own threads on a real
session.

## Display-free execution evidence

Both packaged binaries load and run. Pointed at a GTK backend they cannot reach,
each reaches its display check and aborts, which proves the payload and its
library resolution are complete without opening a window:

```text
env -u WAYLAND_DISPLAY GDK_BACKEND=x11 DISPLAY=:99 usr/bin/fontcarver-desktop
thread 'main' panicked at tao-0.35.3/src/platform_impl/linux/event_loop.rs:217:53:
Failed to initialize gtk backend!: BoolError { message: "Failed to initialize GTK" }
exit 101
```

The same result, with the same exit status, for
`FontCarver_0.1.0_amd64.AppImage` and for `GDK_BACKEND=broadway`. The AppImage
case is the more informative of the two: it mounts itself, runs linuxdeploy's
`AppRun` and the GTK hook, resolves its own bundled GTK and WebKitGTK, and only
then fails on the display — so the bundled runtime is functional, not merely
present. The FUSE mount was released each time and no process was left behind.

An exit 101 with a panic is the current behaviour, not the desired one; see
finding 3.

## Recommended row status

| Row              | Recommended | Exactly what is missing                                                                                                                                                                                                                        |
| ---------------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 9                | Partial     | The packaging half now has real Linux evidence. No macOS or Windows binary of this code exists, so cross-platform parity has no native execution evidence on two of three platforms. Keymap parity is a separate UI concern, not covered here. |
| 62               | Partial     | A Linux application package is built, inspected and executed for the first time. Installation is verified by extraction, not by `apt install`, because that needs `sudo`. macOS and Windows installation remain unverified.                    |
| P5 release block | Partial     | Definitions, helpers, licensing audit, docs and now real Linux packages are all in place. Publication still needs the AppImage payload licensing cleared, the macOS and Windows runs executed, and signing and notarization set up.            |

Row 62's earlier evidence deserves a correction: P4 recorded a "temporary Linux
install", but that was an install of _generated font files_ into
`~/.local/share/fonts`, not of the FontCarver application. This record is the
first application-package evidence for the row.

## Remaining limits

- The AppImage is not redistributable until the 73 bundled packages that name a
  copyleft license have per-file rulings, their notices travel inside the image,
  and the LGPL relinking and source-offer obligations for the GTK, WebKitGTK and
  GStreamer core are satisfied. Prefer the deb until then.
- The deb still has no `changelog.Debian.gz`, which `lintian` reports for a
  vendor package. `bundle.linux.deb.changelog` can supply one once there is a
  release history worth writing; it is a warning, not the policy error that the
  missing copyright document was.
- `bundle.licenseFile` names `LICENSE-MIT` alone, because WiX's license dialog
  takes a single document. Both licenses are installed, and the Debian copyright
  document names both, so this affects the Windows installer dialog only.
- The generated notice bundle is mounted at `licenses/dependencies/`, so the
  project's own licenses appear at `licenses/dependencies/LICENSE-MIT` as well as
  `licenses/LICENSE-MIT`, and the per-package notices sit one level deeper at
  `licenses/dependencies/dependencies/`. It is correct and verified, just
  awkwardly named; changing it costs a full rebuild for no user benefit.
- No `.fcf` file association is declared, so a FontCarver document cannot be
  opened from a file manager. This is real packaging work (a MIME type, an icon
  and `Exec` argument handling) and is outside this task's work list.
- Two tests in the bundle verifier remain necessarily host-bound: the DMG and MSI
  paths cannot run on Linux at all, and the verifier refuses them by design.

## Resources

The release build used its own Cargo target directory outside the workspace, so
none of it landed in the shared development cache. After the build it held
3.4 GiB. Work item 6 allows keeping a release directory under 6 GiB, but the
shared build budget went over its ceiling while this task was finishing, so the
regenerable parts were removed anyway: `release/deps`, `release/build`,
`release/.fingerprint`, `release/incremental`, `release/examples`, the `debug`
tree the API generator produced, the generated license staging directory and the
leftover `FontCarver.AppDir` beside the AppImage. That freed **3.1 GiB** and left
**271 MB**: the two installers, their manifest and the built executable. Both
installer digests are unchanged after the cleanup and still match the ones
recorded above.

The shared budget itself needs attention, and it is not this task's to fix. At
13:48 the checker reported:

```text
build: 20.81 GiB, 30,539 files
errors: build artifacts exceed the capacity ceiling
FAIL — pause builds and inspect disposable artifacts
```

which is what stopped the last rebuild attempt before it compiled anything. After
this task's cleanup it passes again, at 19.58 GiB — still inside the 10 GiB
warning band and a few hundred megabytes from the ceiling, with seven agents
still building:

```text
build: 19.58 GiB, 23,564 files
temporary: 0.15 GiB, 7,835 files
workspace free: 652.43 GiB
warnings: build artifacts exceed 10 GiB; plan profile-cache cleanup
PASS
```

`target/debug/deps` alone was 18 GiB, which is the shared development cache the
concurrent test builds share; clearing it would have destroyed other tasks' work,
so it was left alone. Three stale directories from finished phases looked like the
cheapest safe reclaim for whoever owns that decision —
`target/p1-acceptance` (509 MB), `target/p2b-planning` (21 MB) and
`target/p2b-agent-browser-profile` (21 MB) — but they belong to earlier phases
rather than to this task, so they were not touched either.

Shortly after, another task cleared the shared development profile and the budget
came back to 9.60 GiB with no warnings at all. That removed the disk reason to
stop rebuilding, so the frontend was re-checked on its own, without taking the
cargo lock:

```text
pnpm --dir app check
errors in FeatureDraftSession.test.ts, FeatureSourceEditor.test.tsx,
GlyphVariationPanel.test.tsx and others — missing FeatureGenerationSettings
fields, a missing VariationPreview.shape_status, absent editor props
ELIFECYCLE Command failed with exit code 2
```

Other tasks' test files are still being brought in line with their changed types,
so a release build would still stop at `tsc -b` within a minute. The rebuild
therefore stays for the wave's final pass — and this is the cheap way to decide
whether attempting one is worth the shared lock at all: run `pnpm --dir app check`
first, since that is the gate the packaging build hits.
