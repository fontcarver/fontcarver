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
