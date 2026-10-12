# Release channels: bundle targets and the copyleft policy

**Date:** 2026-10-07
**Status:** design, awaiting review

## 1. Decisions this implements

Settled by the project owner on 2026-10-07:

- Add **rpm**.
- Ship the **stock Tauri AppImage**. Drop the thin-AppImage recipe.
- Windows ships **nsis** plus a **portable zip** that runs from an extracted
  folder. MSI is dropped: two Windows installers on a download page asks users a
  question they cannot answer.
- macOS is unchanged (`dmg`).

Ruling on licensing, also from the owner: LGPL libraries bundled into an AppImage
do not infect FontCarver's own license, and crediting them in a notice is the
right shape of remedy. §3 confirms the first half, and completes the second.

Non-goals: Flatpak, Arch/AUR, code signing, notarization, macOS `app` target.

## 2. Target matrix

| Platform | Today | After |
| --- | --- | --- |
| Linux | `deb` (+ gated thin `appimage`) | `deb`, `rpm`, `appimage` (stock) |
| macOS | `dmg` | `dmg` |
| Windows | `msi` (2 artifacts, one per WiX language) | `nsis` (1, multi-language) + `portable.zip` |

Tauri v2's full target set is `deb`, `rpm`, `appimage`, `app`, `dmg`, `msi`,
`nsis`, and `bundle.targets` defaults to `"all"`. There is no portable target, so
the zip is ours to build (§6). `tauri.conf.json` declares `"targets": "all"` but
`build_release.py` always passes an explicit `--bundles`, so the declaration never
takes effect; leave it and change the script's map.

## 3. The copyleft policy, corrected

### 3.1 What is true

**No infection.** GTK, GLib, WebKitGTK and GStreamer core are **LGPL-2.1**, not
GPL. LGPL exists precisely to permit linking from differently licensed programs,
so bundling them does not change FontCarver's `MIT OR Apache-2.0`. The application
also sits inside the AppImage payload while the runtime merely mounts it and
`exec`s — mere aggregation, no linking relationship. `fusermount3` is GPL-2 but is
invoked as a separate process, which crosses a process boundary, not a linking
one. Verified on a current host: `/usr/share/doc/libfuse3-4/copyright` records
`Files: lib/*` as LGPL-2.1 and `Files: *` as GPL-2.

The current `docs/licensing.md` §5.1 reads as though this were a legal bar. It is
not, and that wording is corrected.

### 3.2 What a credit line does not cover

LGPL-2.1 §6 asks for three things from whoever **redistributes** a copy of the
library. A credit covers the second:

1. the LGPL license text — already in-repo and hash-pinned as
   `scripts/release_licenses/native/HOST-LGPL-2.txt` and `HOST-LGPL-2.1.txt`;
2. notice that the library is used, naming it and its version;
3. **source availability** — either the library's source, or a written offer
   valid three years. This is the part a credit line does not discharge.

A fourth condition, that the user be able to replace the library, is satisfied
mechanically by dynamic linking.

So the remedy is the owner's remedy plus a source pointer. Both are cheap, and
§5.2 generates them from a tool the repository already has.

### 3.3 The one gate that stays: GPL is not LGPL

GPL has no linking exception, so a bundled `.so` that is GPL rather than LGPL is a
genuine distribution problem. `THIRD_PARTY.md` §3.4 records the measured payload:
**177 shared objects from 126 Debian packages**, 73 naming a copyleft license,
notices shipped for **10**; the 2026-10-02 rerun found 129 candidate packages, 75
copyleft labels and **30 with no machine-readable license label**.

Two facts make that smaller than it looks:

- Those labels come from Debian *source* packages. A copyleft label may belong to
  tests or tools that were never shipped, so it is a review lead, not a per-binary
  ruling. §5.1 already says this and keeps saying it.
- **`bundleMediaFramework` is not enabled** in `tauri.conf.json` (the
  `bundle.linux.appimage` block is absent, so it defaults false), so GStreamer is
  not bundled. That is the single case Tauri's own documentation warns about:
  "GStreamer plugins in the `ugly` package are licensed in a way that may make it
  hard to distribute them as part of your app." The main GPL vector is already off.

### 3.4 The rule after this change

`docs/licensing.md` §5 changes from *no copyleft in a shipped artifact* to:

- **Application dependencies** (Cargo and npm graphs linked into FontCarver):
  unchanged — permissive only, no copyleft, MPL-2.0 only by recorded ruling.
- **Native payloads copied in by a bundler:** LGPL is permitted, provided the
  §3.2 materials ship with the artifact. **GPL, AGPL and SSPL are not permitted**,
  and an unidentified license is treated as not permitted until identified.
- **Never** enable `bundleMediaFramework` without re-running the §5.2 scan.

## 4. Deletions

The thin-AppImage recipe exists only to satisfy the old blanket ban and is dropped:

- delete `scripts/build_thin_appimage.py` and `scripts/test_build_thin_appimage.py`;
- delete the thin-builder branch in `scripts/build_release.py`, including
  `--appimage-sysroot` and the "build a deb solely to feed the thin builder"
  coupling;
- remove `require_releasable_appimage`'s thin-recipe verification from
  `scripts/check_release_bundle.py`'s import path.

`scripts/appimage_policy.py` and `scripts/appimage_inventory.py` are **kept and
repurposed** (§5). Keeping them is the point: they already know how to walk a
payload, and they become the notice generator and the GPL gate.

Independently of the policy, the thin recipe was also already broken: it pins
`libsquashfuse0 (= 0.5.0-2build1)` and asserts `pkg-config --modversion squashfuse
== '0.5.0'`, and its `RUNTIME_NEEDED` wants `libfuse3.so.3`. A current Ubuntu host
has squashfuse 0.5.2 and `libfuse3-4` providing `libfuse3.so.4`, with no
`libfuse3.so.3` at all.

## 5. AppImage: notices and the GPL gate

### 5.1 Notice generation — `scripts/appimage_inventory.py`

The tool already records every payload path with its hash plus current-host Debian
package ownership candidates. It gains an output mode that writes, into the image
at `licenses/appimage/`:

- `BUNDLED-LIBRARIES.txt` — one row per bundled shared object: file path, owning
  package and version, license as labelled, and the `deb-src` source URL for that
  exact package version;
- the LGPL-2 and LGPL-2.1 texts, copied from
  `scripts/release_licenses/native/` with their existing hash checks;
- `SOURCE-OFFER.txt` — a written offer valid three years, naming a contact, for
  any bundled library whose source URL could not be resolved automatically.

The existing caveat stays in the output: host package ownership is a hint, not
build provenance.

### 5.2 The GPL gate — `scripts/appimage_policy.py`

`APPIMAGE_RELEASE_BLOCKERS` and the thin-recipe receipt checks are replaced by a
scan over the built payload:

1. every bundled `.so` resolves to a package with an identified license;
2. no resolved license is GPL, AGPL or SSPL — **LGPL passes**;
3. any `.so` with no identifiable license fails the release, named individually;
4. `licenses/appimage/` exists and lists every bundled `.so` the scan found.

Point 3 is the one that keeps this honest: the 30 unlabelled packages from §3.3
either get identified or block the release. That is a smaller, more precise gate
than the ban it replaces, and it is the part of the old machinery worth keeping.

`appimage_policy.py` has **no dedicated test** today; only
`test_appimage_inventory.py` touches it indirectly. Changing its contract without
one is not acceptable, so `scripts/test_appimage_policy.py` is part of this work.

## 6. Windows portable zip

No Tauri target produces this, so it is a build step over the compiled artifacts:
the executable, its bundled resources (`licenses/`, `INSTALL.md`) and a
`PORTABLE.txt`, zipped with a top-level versioned directory so extraction cannot
scatter files into the user's current folder.

Two problems make this more than zipping:

**WebView2.** `tauri.conf.json` currently sets
`webviewInstallMode: {type: "downloadBootstrapper", silent: true}`. A zip has no
installer, so the runtime cannot be fetched at install time. Two options:

- *Require it present.* Preinstalled on Windows 11 and usually present on Windows
  10 via Edge, but not guaranteed. The app must detect its absence and say so
  clearly instead of failing opaquely.
- *Bundle it* with `webviewInstallMode: {type: "fixedRuntime", path: …}`, which
  adds roughly 180 MB. Microsoft's Fixed Version distribution terms permit this.

Recommendation: require it present, detect and report clearly. A 180 MB portable
zip defeats the purpose, and the detection path is needed regardless.

**Data directory.** Tauri writes configuration under `AppData` by default, which
is not portable behaviour. A real portable build must detect a marker file beside
the executable and redirect its data directory there. **This is application code,
not packaging** — it touches wherever the app resolves its config and cache paths,
and it needs its own tests covering both modes.

## 7. Other work

`scripts/build_release.py`
- `native_bundles` → `{"Linux": ["deb", "rpm", "appimage"], "Darwin": ["dmg"], "Windows": ["nsis"]}`, with the portable zip produced after the Windows bundle step.
- Delete the `msi_languages` parameter and the WiX language lookup at :167;
  `validate_built_artifacts` then expects one artifact per format.
- `validate_versions` keeps its numeric `major.minor.patch` rule but stops
  describing it as an MSI limit. Do not widen version rules as a side effect.

`scripts/check_release_bundle.py` — format awareness is hardcoded across `HOSTS`,
`extracted()`, `platform_metadata()`, `inspect_payload()` and `signatures`. Add
`rpm` (extract via `rpm2cpio | cpio -idmv`, metadata via `rpm -qp`), `nsis` (a PE,
so the same `MZ` signature `msi` used, but extraction uses `7z x`, not
`msiexec /a`) and `portable` (a zip, read with `zipfile`); remove `msi`. The
`appimage` branch stays and now inspects a stock image.

`tauri.conf.json` — replace `bundle.windows.wix` with `bundle.windows.nsis`
(`languages: ["English", "SimpChinese"]` keeps the current en-US/zh-CN coverage in
one installer); add `bundle.linux.rpm` beside the existing `deb` block, mirroring
its copyright file placement. Do not add a `bundle.linux.appimage` block — its
absence is what keeps `bundleMediaFramework` off (§3.3).

**Install verification.** `scripts/check_deb_install.py` is a full rootless
install/reinstall/remove check in a bubblewrap overlay built around dpkg and apt.
`docs/STATUS.md` treats build + inspection + installation + startup evidence as
the release standard, so `rpm` needs an equivalent. This is the largest single
item in this spec.

**Documentation.** `docs/licensing.md` §5 rewritten per §3.4 and §5.1 per §3.1.
`THIRD_PARTY.md` §3.4 keeps its inventory numbers, gains §3.3's reading guidance,
and replaces the thin-recipe paragraph with the notice mechanism.
`HOST-LIBRARIES.txt` keeps GTK/WebKit for the deb and drops FUSE/squashfuse.
`docs/releasing.md`, `docs/install.md`, `docs/STATUS.md`: new artifact set, and
the portable zip's WebView2 requirement.

## 8. Plans

Four plans. Plan A is a prerequisite for B only in that B's gate calls A's scan.

- **Plan A — policy and AppImage notices** (§3, §4, §5, licensing docs). Unblocks
  shipping an AppImage at all.
- **Plan B — rpm and nsis** (§7 except install verification).
- **Plan C — rpm install verification** (§7). Separable and large.
- **Plan D — Windows portable** (§6). Contains application code, so it is not a
  packaging-only task and should not be folded into B.

## 9. Testing

Python: `test_appimage_policy.py` new, covering that LGPL passes, GPL fails, an
unidentifiable license fails and is named, and that a missing
`licenses/appimage/` fails; `test_appimage_inventory.py` extended for the notice
output; `test_check_release_bundle.py` extended for `rpm`, `nsis`, `portable` and
the removal of `msi`; `test_build_release.py` and `test_release_windows_job.py`
updated for the new target map.

Application: portable data-directory resolution, both modes, including a marker
file beside a symlinked executable.

Artifact evidence, per the repository's existing standard: build each format,
inspect its payload, install it, start it. For the AppImage, the decisive evidence
is that `licenses/appimage/BUNDLED-LIBRARIES.txt` accounts for every `.so` the
scan found and that no row is GPL or unidentified. For the portable zip, extract
and launch on a Windows host, and separately confirm the no-WebView2 message on a
host without it.

## 10. Risks

- **The notices have no build-time caller** (found while implementing §5, 2026-10-08).
  Tauri seals the AppImage in one step, so `write_notices` cannot run "on the
  extracted payload" as §5.1 assumed: injecting afterwards needs an extract/repack
  step, and whether Tauri leaves a reusable intermediate AppDir cannot be settled
  without running a real build. `appimage_inventory.py --notice-dir` therefore exists
  but nothing in `build_release.py` calls it, so a stock AppImage reaches the gate
  with no notice directory and is **blocked** — the safe direction, and the designed
  behaviour for an un-noticed image, but the `appimage` target cannot ship until this
  is resolved. Two candidates for the real-build task: (a) write notices into Tauri's
  intermediate AppDir if it survives, (b) add an extract/inject/repack post-step using
  appimagetool from Tauri's cache. The deb is unaffected.

- **~~The 30 unlabelled packages~~ — measured 2026-10-08; it is 90, not 30.**
  See [the census](../../appimage-bundled-license-census.md). Running the gate's
  classifier over the recorded real-build inventory gives 40 pass, 57
  ruling-required, 30 unidentified and **3 forbidden**. The three forbidden ones
  (`libwmf-0.2-7`, `libwmf-0.2-7-gtk`, `libwmflite-0.2-7`, AGPL-3) are genuinely in
  the payload as a gdk-pixbuf WMF loader and cannot be ruled around, so the
  `appimage` target needs payload surgery as well as notices. Only 3 of the 30
  unidentified resolve from current host metadata, and 7 of their copyright files
  changed within six days of the capture, so the determination set drifts with the
  build host and the census has to be re-run per release.
- **`deb-src` URL stability** for the exact bundled package versions is not
  verified; if it proves unreliable, the written offer in §5.1 carries more of the
  load, which creates a three-year obligation someone must own.
- **NSIS payload extraction** with `7z x` works on most NSIS installers but is not
  as uniform as MSI administrative install; verify against a real artifact before
  relying on it for evidence.
- **rpm install verification** may not fit the bubblewrap overlay approach, which
  assumes dpkg state layout.
- **Portable WebView2 detection** must be verified on a Windows 10 host without
  the runtime, which the project does not currently have evidence for; STATUS.md
  already notes macOS and Windows installation is unverified.
