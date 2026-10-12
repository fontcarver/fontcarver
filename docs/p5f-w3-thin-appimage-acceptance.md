# Thin AppImage acceptance — 2026-10-02

Status: the final integrated Linux AppImage from commit `f6f6cf9` passes the
corrected artifact gate, reversible `DEB` → `APP` metadata check, actual FUSE and
extraction probes, expected no-display refusal, and isolated virtual-display
GUI-process survival. Eleven builder tests and 57 combined release tests pass.
This completes the local thin-AppImage scope; signing, publication and native
interactive/cross-platform parity are not claimed.

## What changed

`scripts/build_thin_appimage.py` constructs a Linux x86_64 image from an inspected
FontCarver deb. It copies the application, icons and notices, then adds a runtime
built from checksum-pinned upstream source. It rejects additional ELF binaries,
shared libraries, escaping links, unexpected runtime dependencies and linked
static archive members. Only the application ELF's unique three-byte bundle tag
changes from `DEB` to `APP`; its original and resulting hashes are recorded.
Source, bounded source changes, notices, compiler command,
link map and input hashes travel inside the image. The builder publishes nothing.

The standard static AppImage runtime embeds LGPL FUSE. The historical AppImageKit
13 alternative is explicitly obsolete upstream. Instead this candidate compiles
the current MIT/public-domain source from
[type2-runtime 8f39b89e2ac31e1640b3d3f7e9a5108e6ce805fa](https://github.com/AppImage/type2-runtime/tree/8f39b89e2ac31e1640b3d3f7e9a5108e6ce805fa)
against host shared libraries. Its `DT_NEEDED` set is exactly `libsquashfuse.so.0`,
`libsquashfuse_ll.so.0`, `libfuse3.so.3`, and `libc.so.6`. The
[squashfuse 0.5.0 private header](https://github.com/vasi/squashfuse/blob/0.5.0/fuseprivate.h)
is BSD-2-Clause and is included with its notice. The runtime's complete source
preserves Simon Peter, Alexander Larsson and public-domain MD5 attribution.
The image also carries the checksum-pinned LGPL-2.1 text distributed with
[libfuse 3.14.0](https://github.com/libfuse/libfuse/blob/3f65bb699cd2f0afe2b9519c39d29922229f2168/LGPL2.txt)
and a prominent notice identifying host libfuse3 use and support for compatible
host replacements. This adds a license document, not a bundled library.

The small source changes check bounded path formatting/read results, retry an
interrupted keepalive read, correct static-library claims in help, and remove the
upstream static-runtime marker. GCC compiles the result with `-Wall -Werror`, PIE,
RELRO and immediate binding. No FontForge source was read or adapted.

## Verification

`TMPDIR=<task>/tmp python3 -m unittest discover -s scripts -p test_build_thin_appimage.py`:
11 tests passed, including the three metadata regression tests added after the
initial eight tests. The initial red run failed because the builder did not yet exist.
Tests cover source checksum rejection, exact dynamic linkage, rejection of static
archives and extra payload libraries, symlink boundaries, patch context checks,
launcher paths/arguments containing spaces, and the exact three-byte bundle
conversion with invalid-marker rejection and unchanged input bytes/permissions.

The actual source-built runtime is 49,720 bytes, SHA-256
`7c017539a65934c5ffbe66a805c630323608794748d2a82cd5d88ef321c18f91`.
Building a minimal SquashFS image with the same runtime and executing it verified:

- `--appimage-offset`: exit 0, `49720`.
- Ordinary FUSE mount and launch: exit 0; read the mounted payload and preserved
  the argument `argument with spaces`.
- `--appimage-extract-and-run`: exit 0; identical payload and argument result.
- `--appimage-extract`: exit 0; extracted file contents matched.

The host lacks squashfuse development/runtime packages and squashfs-tools, and
passwordless sudo is unavailable. Authenticated Ubuntu package indexes supplied
`libsquashfuse-dev`, `libsquashfuse0`, `libfuse3-dev`, and `squashfs-tools` into
task-local storage (395 kB download); no system package or apt configuration was
changed. Those libraries were used through temporary compiler/runtime search
paths and are not part of the image payload.

The exact Ubuntu 24.04 amd64 packages and verified SHA-256 values were:

| Package           | Version         | SHA-256                                                            |
| ----------------- | --------------- | ------------------------------------------------------------------ |
| libfuse3-dev      | 3.14.0-5build1  | `cfa8460d23b5143d14bb336b3008784df72a5e9fc32057ec1dc85beadca9aecb` |
| libsquashfuse0    | 0.5.0-2build1   | `19594e096e5772165df1ecb4382f63c1b267cf14703c15ee02011b503d0715b3` |
| libsquashfuse-dev | 0.5.0-2build1   | `7ba23670f18cfea18955ba98d03d630045da82f8a5f9506e68b0411441df8124` |
| squashfs-tools    | 1:4.6.1-1build1 | `87fae263846bab255d4a51ad9fc623685497ad830db60758dde39589c9fdadcb` |

They came from `http://archive.ubuntu.com/ubuntu`, using the authenticated local
Ubuntu apt indexes. Downloading used only `/etc/apt/sources.list.d/ubuntu.sources`
and an empty task-local source-parts directory; the unrelated broken Opera apt
source was not edited. `dpkg-deb --extract` unpacked these tools into task scratch.
The installed host supplied `libfuse3.so.3.14.0` and GCC
`13.3.0-6ubuntu2~24.04.1`.

## Reproduction

On a clean Ubuntu 24.04 development/CI host, install the build prerequisites
listed in the builder's module documentation. This host instead needs the
following isolated equivalent, run from the repository root (no sudo):

```sh
thin_task="$PWD/.scratch/p5f-w3-thin-appimage"
mkdir -p "$thin_task/packages" "$thin_task/root" "$thin_task/empty-sources" "$thin_task/tmp"
cd "$thin_task/packages"
apt-get -o Dir::Etc::sourcelist=/etc/apt/sources.list.d/ubuntu.sources \
  -o Dir::Etc::sourceparts="$thin_task/empty-sources" download \
  libfuse3-dev=3.14.0-5build1 libsquashfuse0=0.5.0-2build1 \
  libsquashfuse-dev=0.5.0-2build1 squashfs-tools=1:4.6.1-1build1
```

Verify the four hashes above, extract each downloaded deb with
`dpkg-deb --extract PACKAGE "$thin_task/root"`, and return to the repository.
The builder downloads checksum-pinned upstream inputs and verifies nine approved
source/license/notice files; it does not trust a prebuilt runtime. To compile only the
runtime again, the exact Python entry point used was:

```python
import os, pathlib, sys
sys.path.insert(0, 'scripts')
from build_thin_appimage import build_runtime
task = pathlib.Path('.scratch/p5f-w3-thin-appimage').resolve()
scratch = task / 'runtime-check'
scratch.mkdir()
runtime, receipt = build_runtime(
    scratch, scratch / 'notices', task / 'root',
    dict(os.environ, TMPDIR=str(scratch)))
print(receipt)
```

The recorded minimal runtime test created a two-file AppDir: executable `AppRun`
printed its first argument and read `"$APPDIR/probe.txt"`; `probe.txt` contained
`mounted payload`. Command
`mksquashfs APPDIR probe.squashfs -noappend -comp gzip -processors 2`
created the filesystem; byte-concatenating the built runtime and that filesystem
and setting mode `0755` produced the test image. With `TMPDIR` in task scratch and
`LD_LIBRARY_PATH="$thin_task/root/usr/lib/x86_64-linux-gnu"`, the four commands
listed under Verification above all passed. No native desktop interaction was
performed.

The integrated-candidate command executed after the coordinated build gate was:

```sh
python3 scripts/build_release.py --bundles deb,appimage \
  --appimage-sysroot "$thin_task/root"
```

The wrapper supplies those isolated tool/library paths only to the thin builder
and its artifact verifier. A separately built fresh deb can also be passed to
`scripts/build_thin_appimage.py --deb PATH --output PATH.AppImage --sysroot PATH`,
with the extracted `usr/bin` on `PATH`; inspecting/running the result on this host
also needs that temporary `LD_LIBRARY_PATH`. The actual artifact and startup
results from this execution are recorded below.

## Final integrated application candidate

The packaging agent built `target/release/bundle/appimage/FontCarver_0.1.0_amd64.AppImage`
from application commit `f6f6cf9ce481d11314e25c540b5719fd0951a00f`. Its
[release manifest](acceptance/p5f-w3-packaging/release-manifest.json) records the
stable source fingerprint, deb and AppImage inspection. This agent independently
ran the actual AppImage, preserving
[startup/extraction evidence](acceptance/p5f-w3-thin-appimage/candidate-probes.json),
the embedded [build receipt](acceptance/p5f-w3-thin-appimage/BUILD.json) and
[link map](acceptance/p5f-w3-thin-appimage/runtime.map).
The exact task-local
[runtime probe recipe](acceptance/p5f-w3-thin-appimage/verify_candidate.py.txt) and
[virtual-display probe recipe](acceptance/p5f-w3-thin-appimage/verify_gui_candidate.py.txt)
are archived as evidence. To repeat them, recreate the isolated prerequisites,
copy each recipe back to its original `.scratch/p5f-w3-thin-appimage/` location
without the `.txt` suffix, then run it with the AppImage path as its argument.

| Item                                   | Result                                                                           |
| -------------------------------------- | -------------------------------------------------------------------------------- |
| AppImage size                          | 23,683,640 bytes                                                                 |
| AppImage SHA-256                       | `3a9029b1aa83d39d78365b18202b39aa5a035f640083dfb4e53151c205793536`               |
| Input deb SHA-256                      | `e01096f3abd387e5758dc177cfe5a85e4ec5030613d1f3f1c890aac789e429de`               |
| Input deb application ELF SHA-256      | `9715a2ad24ecaa9975cea03a56cf68ffe2de95931a569884b900000980fa976e`               |
| AppImage application ELF SHA-256       | `c8fe34032a54e4e609ae0e641f0c149becc67dbef66c2086d1059187066f9432`               |
| Bundle metadata                        | Unique `APP`; only three bytes differ from the deb, at offset 4,743,271          |
| Runtime prefix                         | 49,720 bytes; `7c017539a65934c5ffbe66a805c630323608794748d2a82cd5d88ef321c18f91` |
| Runtime version/offset                 | Both exit 0; approved commit and exact receipt offset                            |
| Normal FUSE `--version`                | Exit 0; `fontcarver-desktop 0.1.0`                                               |
| FUSE without a display                 | Exit 1 promptly; deliberate `no graphical session is available` diagnostic       |
| `--appimage-extract-and-run --version` | Exit 0; `fontcarver-desktop 0.1.0`                                               |
| Extract-and-run without a display      | Exit 1 promptly; same deliberate diagnostic                                      |
| `--appimage-extract`                   | Exit 0; receipt and application ELF recovered and hashed                         |

No `APPIMAGE_EXTRACT_AND_RUN` environment variable was present during the normal
FUSE probes; the mounted path was exercised separately from extraction. All
configuration/cache/data directories belonged to this task. No processes with
those private configuration paths and no candidate mounts remained after tests.

Independent artifact review re-extracted the final image, verified the corrected
gate and nine source/license pins, and checked the actual deb against the
receipt's explicit three-byte application transformation. Other original deb
payload files remain unchanged. The earlier `6e35f70` candidate's runtime/startup
evidence is retained only under
[superseded-deb-marker](acceptance/p5f-w3-thin-appimage/superseded-deb-marker/candidate-probes.json);
its retained `DEB` tag is the defect described below, and that image is superseded.

## Isolated graphical startup

The host initially lacked Xvfb. Two additional official Ubuntu packages were
downloaded and hash-checked into the existing task sysroot, with no system
installation: `xvfb` and `xserver-common`, both `2:21.1.12-1ubuntu1.8` (912 kB
combined download). Their exact hashes and all other temporary test prerequisites
are retained in [package provenance](acceptance/p5f-w3-thin-appimage/host-test-packages.json).

Xvfb ran with `-displayfd`, `-screen 0 1280x900x24`, `-nolisten tcp`, `-noreset`
and a private Xauthority file. It selected the unused display `:2`; `xdpyinfo`
against that private display passed. The existing desktop displays were never
connected to and no input events were sent. The actual AppImage was launched
through normal FUSE with that display and default application security settings.
It remained alive for **15.065 seconds**, then the probe deliberately terminated
its owned process group with SIGTERM. The virtual server exited 0 afterward.

[GUI probe evidence](acceptance/p5f-w3-thin-appimage/gui-probes.json) records
`timed_out: true` and `exit_code: -15`: the observation interval expired and the
test stopped the still-running app. This is the intended liveness result, not
an application crash. Only non-fatal DRI3 acceleration warnings appeared. Xvfb
also reported non-fatal key-symbol warnings. There was no sandbox-disable
override, live-desktop interaction or input automation.

This proves native loader/GUI-process startup on the virtual display. It does
**not** establish that visible frontend contents rendered correctly or that
native interactive editing workflows passed. Those claims require separate
evidence and are not inferred from either liveness or the no-display refusal.

## Scope and remaining checks

This is a host-dependent package for a compatible Linux x86_64 system, tested on
Ubuntu 24.04 with squashfuse 0.5.0 and FUSE 3. Recipients still need the deb's
GTK/WebKit dependencies, `libsquashfuse0`, `libfuse3-3` and `fuse3`. The extraction
fallback avoids mounting; it still needs those host libraries. Universal Linux
portability, code signing and public distribution are not claimed. Build receipts
provide inspectable evidence, not protection against deliberate forgery.

The integrated application build, actual candidate gate inspection, no-display
startup and virtual-display liveness now pass. Interactive native GUI workflows,
other distributions/platforms, signing and publication remain outside this
report's verified boundary.

## Review round

The independent release reviewer verified the source/notice pins, exact source
patch output, actual runtime hash, four dynamic dependencies and compiler link
map; the 8 builder tests and 16 inventory/policy tests passed in that review.
The license reviewer requested the host LGPL notice/copy described above; the
resulting 9 approved source/license/notice files have matching content hashes.
The runtime bytes are unchanged by that notice addition.
The final `f6f6cf9` application image was independently re-extracted and verified
against the actual deb, source fingerprint, all nine pins and notices, runtime
prefix, link map and retained probe records. Its unique `APP` marker differs
from the deb's unique `DEB` marker by exactly the intended three bytes; reversing
those bytes recovers the actual deb executable. Independent final version and
no-display probes also passed without timing out. The reviewer found no remaining
blocker and confirmed that the runtime prefix matches the reviewed prototype.

## Bundle metadata correction

Tauri-utils 2.9.3 `platform::bundle_type` maps
`__TAURI_BUNDLE_TYPE_VAR_APP` to AppImage and the `DEB` marker to Debian. Reusing
the deb executable without changing this tag therefore misidentified the first
image. `patch_bundle_type` now requires a regular ELF with exactly one marker
prefix and the expected `DEB` suffix. It changes only those three bytes to `APP`
inside the extracted application, preserving permissions and the input deb.
The receipt records `application.source_sha256`, `appimage_sha256`,
`marker_offset`, `before` and `after`.

The three new builder tests first failed, then passed with the fix: **11 builder
tests pass**. Missing, duplicate, conflicting, truncated and already-converted
markers are rejected without modification; non-ELF and symlink inputs are also
rejected. A real extraction of the first deb confirmed exactly three changed
bytes at offset **4,740,623**, unchanged permissions and unchanged input deb
SHA-256. Its application hash changes from `d96b55bc…` to `9c79ce4f…` solely by
that transformation; the full hashes are retained in the
[real-deb regression record](acceptance/p5f-w3-thin-appimage/bundle-marker-regression.json).
The AppImage gate checks the actual `APP` marker and hashes
the in-memory reverse transformation back to `DEB`; the deb gate separately
requires its own unique `DEB` tag. The independent reviewer ran **57 combined
tests**, all passing. The first candidate files were not patched in place. The
fresh `f6f6cf9` candidate was rebuilt and validated separately; its final marker
offset and both application hashes appear in the table above and embedded receipt.

## First tooling checkpoint and cleanup

At the first tooling checkpoint, the user's instruction to finish the current
repair and stop deferred the planned full candidate build. Builder tests:
**8 passed**. No owned processes or mounted images remained. All 2,193,520 bytes of task scratch,
including downloaded packages, extracted sysroot, runtime and probe images, were
removed; the reproduction instructions above retain the necessary evidence.

`python3 scripts/check_artifacts.py` at that checkpoint:

```text
build: 16.18 GiB, 10,852 files
temporary: 0.04 GiB, 1,756 files
workspace free: 605.62 GiB
warnings: build artifacts exceed 10 GiB; plan profile-cache cleanup
PASS
```

Other agents still owned active build/test work at this checkpoint. This task
neither cleared their caches nor treated their remaining temporary data as its
own. After the user renewed authorization, the four packages and nine approved
inputs were reacquired and checked against the hashes above. They were retained
in the same task-local directory through the coordinated Linux builds and final
validation, with two additional isolated Xvfb packages for the GUI startup probe.

## Final cleanup

The packaging, AppImage-policy and independent-release reviewers confirmed they
had finished using the isolated sysroot. No task-owned processes or mounts
remained. The complete `.scratch/p5f-w3-thin-appimage` directory was then removed:
**78,625,253 bytes in 1,254 regular files**, including the isolated dependencies,
extracted application, private Xvfb data and temporary probe files. The final
deb/AppImage in `target/release/bundle` and the small acceptance records remain.
Source files, shared caches and other tasks' data were not removed.

`python3 scripts/check_artifacts.py` after final cleanup:

```text
build: 5.33 GiB, 12,262 files
temporary: 0.00 GiB, 1 files
workspace free: 616.36 GiB
PASS
```

The temporary total immediately before cleanup was 0.08 GiB / 1,264 files;
concurrent owners also completed their cleanup during this checkpoint. Final
`git status` inspection confirmed this task's remaining changes are its acceptance
records and task documentation; application and packaging source stay frozen.
