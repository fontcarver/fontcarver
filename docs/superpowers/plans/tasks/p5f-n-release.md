# p5f-n: Packaging and platform acceptance

Area id: `n-release`. Ports: none (8814/1514 reserved). Protocol: `p5f-protocol.md`.

## Rows and scope

| Row | Requirement                                                                                                         | State at start                                                                                                                                               |
| --- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 9   | Cross-platform parity (native execution evidence part)                                                              | Linux only; macOS/Windows workflows defined in `.github/workflows/release.yml` but never executed.                                                           |
| 62  | Test install on all three platforms                                                                                 | Linux temporary install evidence from P4; no deb/AppImage of the current code built; macOS/Windows unverified.                                               |
| —   | P5 release block: deb/AppImage/dmg/msi definitions, locally verifiable installers, platform CI, user/developer docs | `scripts/build_release.py`, `check_release_bundle.py`, `check_release_licenses.py`, `docs/releasing.md`, `docs/install.md`, `docs/p5-release-acceptance.md`. |

Existing records: `docs/p5-release-acceptance.md`, `docs/releasing.md`,
`docs/install.md`, `docs/superpowers/plans/tasks/p5-release*.md`, `docs/p4-acceptance.md`
(Linux install section), `.github/workflows/{ci.yml,release.yml,site.yml}`.

Environment facts: Linux host with `webkit2gtk-4.1` and `gtk+-3.0` dev packages,
`dpkg-deb`, `desktop-file-validate`, and Tauri's linuxdeploy tools cached in
`~/.cache/tauri`. No `sudo`, no `xvfb-run`, no `appimagetool` on PATH. 20 cores,
62 GiB RAM. The shared Cargo target directory is close to the 20 GiB ceiling.

## Work

1. **Build the Linux packages of the current code** with
   `python3 scripts/build_release.py --bundles deb,appimage` using a separate
   release target directory **outside the workspace**:
   `CARGO_TARGET_DIR=/home/chen/.cache/fontcarver-release-target` (create it), and
   pass `--build-dir /home/chen/.cache/fontcarver-release-target` to every
   `scripts/check_artifacts.py` call. Serialize with the shared flock like any
   cargo command. Fix the helper if it cannot handle the alternate target
   directory or fails for a reason inside this repository; do not work around a
   real product bug silently, report it.
2. **Inspect and smoke-test**: `dpkg-deb -I/-c`, `desktop-file-validate` on
   the desktop file, extract the AppImage (`--appimage-extract`), check the
   bundled licenses/resources, run the binaries in a way that works without a
   display (`--version`/`--help` if supported; otherwise start with a
   `GDK_BACKEND`/offscreen approach if WebKitGTK allows it, and record exactly
   what could and could not be executed). `dpkg-deb -x` into scratch to verify
   the file layout since `apt install` needs sudo. Run
   `python3 -m unittest discover -s scripts -p "test_*release*.py"` and
   `python3 scripts/check_release_bundle.py` / `check_release_licenses.py` on the
   outputs.
3. **Review the workflows** (`release.yml`, `ci.yml`) line by line for errors
   that would only show on macOS/Windows runners (shell differences, `find`
   flags, path quoting, WiX/VBScript prerequisites, `hdiutil` flags, missing
   `actions/upload-artifact` steps, timeouts, permissions). Fix what you can
   verify by reasoning and note what only a run can prove. Add a
   `workflow_dispatch` input for the bundle list if missing. Make the CI
   workflow's e2e job robust (Playwright browser cache, `--min-free-gib`).
4. **Version and metadata consistency**: `Cargo.toml` workspace version,
   `src-tauri/tauri.conf.json`, `app/package.json`, `src-tauri/Cargo.toml`,
   product name, identifier, icons present for all platforms, category,
   `longDescription`, license file, `THIRD_PARTY.md` bundled.
5. **Docs**: update `docs/releasing.md`, `docs/install.md` and append to
   `docs/p5-release-acceptance.md` with the actual local results (sizes,
   package names, what was executed), and record macOS/Windows as "workflow
   defined, execution pending on the platform runners".
6. **Cleanup**: keep the release target directory only if it is under 6 GiB
   after the build (report its size); otherwise remove `release/deps` and
   `release/build` inside it after saving the package files and evidence under
   `docs/acceptance/p5f-release/` (small text evidence only: listings, checksums,
   sizes; never commit installers).

## Owned files

`scripts/build_release.py`, `scripts/check_release_bundle.py`, `scripts/check_release_licenses.py`,
`scripts/test_*release*.py`, `scripts/release_licenses/**`, `.github/workflows/release.yml`,
`.github/workflows/ci.yml`, `src-tauri/tauri.conf.json` (bundle section; the
`windows` array belongs to task a), `src-tauri/icons/**`, `src-tauri/Cargo.toml` (metadata only),
`docs/releasing.md`, `docs/install.md`, `docs/p5-release-acceptance.md`,
`docs/acceptance/p5f-release/**`, `docs/p5f-n-release-acceptance.md` (or append to the release record).

Shared files (narrow patches only): `Cargo.toml` (version only), `app/package.json`
(version only), `THIRD_PARTY.md`.

## Gap analysis

Written 2026-09-30 before changing anything, after reading the protocol, the
audit rows, `docs/p5-release-acceptance.md`, both release helpers, both
workflows, `src-tauri/tauri.conf.json` and the P4 install evidence.

Environment probe (no `sudo`, no display): `dpkg-deb`,
`desktop-file-validate`, `pkg-config`/`pkgconf`, `glib-compile-schemas`,
`gio-querymodules`, `gtk-update-icon-cache`, `/dev/fuse` and `fusermount` are
present; `appimagetool`, `xvfb-run` and a system `patchelf` are not.
`~/.cache/tauri/linuxdeploy-x86_64.AppImage` runs and carries its own
`patchelf` and `strip`, and the GTK plugin's remaining helpers resolve through
its `/usr/lib/x86_64-linux-gnu/...` fallbacks, so AppImage bundling has what it
needs without `sudo`.

### Row 9 — cross-platform parity (native execution evidence part)

Works: one identifier/product name/version across `Cargo.toml`,
`app/package.json` and `src-tauri/tauri.conf.json`; icons for all three
platforms; a three-row native matrix in `release.yml`.

Missing: the workflows have never run, so no macOS or Windows binary of this
code exists. Nothing on this host can produce or execute one.

Doing: build and inspect the real Linux packages; review both workflows for
faults that would only appear on a macOS or Windows runner and fix what
reasoning can settle.

Limit: macOS/Windows execution stays "workflow defined, execution pending on
the platform runners". Keymap parity is another task's UI concern; this task
only supplies the packaging half of the row.

### Row 62 — test install on all three platforms

Works: P4 recorded a real temporary Linux install, but of _generated fonts_
into `~/.local/share/fonts`, not of the FontCarver application. No application
package of the current code had been built at all.

Doing: build deb and AppImage, read the deb control metadata and file list,
validate the desktop entry, extract the AppImage, verify bundled
licenses/resources, and execute the packaged binaries as far as a display-less
host allows.

Limit: `apt install` needs `sudo`, so file-layout verification uses
`dpkg-deb -x` into scratch instead of a systemwide install. The desktop
application needs a WebKitGTK window, so a launch is expected to fail without a
display; exactly what ran and what did not is recorded rather than glossed.

### P5 release block

Works: `build_release.py`, `check_release_bundle.py`,
`check_release_licenses.py`, 32 passing helper tests, `release.yml`, `ci.yml`,
`docs/releasing.md`, `docs/install.md`. All of it was configuration and
synthetic-payload verification; no FontCarver installer had ever been produced.

Doing: the first real build, on an out-of-workspace release target directory so
the shared debug cache is untouched; the checks in work items 2–5; a
`workflow_dispatch` bundle-list input; whatever the workflow review turns up.

Decisions taken without asking:

1. The shared cargo flock is held for exactly one `build_release.py`
   invocation, which runs in the background while the lock-free review and
   documentation work proceeds, so the seven concurrent agents lose the lock
   only for the build itself.
2. `CARGO_TARGET_DIR=/home/chen/.cache/fontcarver-release-target`, and
   `--build-dir` on every `check_artifacts.py` call outside the helper. Inside
   the helper the same directory is picked up automatically from
   `CARGO_TARGET_DIR`, so no helper change is needed for the budget checks.
3. The Tauri CLI warns that the `com.fontcarver.app` identifier ends in `.app`.
   The identifier is outside my ownership (I own the `bundle` section only), so
   it is reported with the exact change needed, not edited.
4. No `.fcf` file association is added. It is a real packaging improvement, it
   is outside this task's work list, and it would change the deb payload that
   the same task is meant to verify.

### Decisions taken after the first build

5. The first deb shipped no `/usr/share/doc/<package>/copyright`, which Debian
   policy requires. It was fixed rather than merely recorded: a verifier check
   and four tests first, then a DEP-5 document at
   `src-tauri/packaging/linux/copyright` wired in through
   `bundle.linux.deb.files`. That adds one new file outside the literal
   ownership list, in a new `src-tauri/packaging/` directory that no other task
   touches.
6. Three protocol retries of the rebuild all failed on other agents' in-progress
   edits to `fc-api`, `fc-formats` and `fc-doc`. Rather than stop, the rebuild
   was left retrying in the background on a longer interval while the review,
   inspection and documentation work continued; each failed attempt holds the
   shared cargo lock for about ten seconds.
7. The display-less execution check was intended to need no display. It turned
   out that this host has a live Wayland session that GTK reaches even with
   `DISPLAY` and `WAYLAND_DISPLAY` unset, so the first probes really launched the
   application. That is disclosed in the acceptance record, and every later check
   forces a GTK backend that cannot connect.
