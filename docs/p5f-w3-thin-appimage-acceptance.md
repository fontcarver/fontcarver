# Thin AppImage acceptance — 2026-10-02

Status: tooling, real runtime feasibility and independent review passed. The
user subsequently authorized completing all remaining work possible on this
machine, including the integrated Linux AppImage build and validation. Its
isolated prerequisites and nine source/license/notice inputs have been restored
and verified; the actual application candidate is awaiting the coordinated build
slot. Audit row 62 remains Partial until its remaining acceptance conditions pass.

## What changed

`scripts/build_thin_appimage.py` constructs a Linux x86_64 image from an inspected
FontCarver deb. It copies the application, icons and notices, then adds a runtime
built from checksum-pinned upstream source. It rejects additional ELF binaries,
shared libraries, escaping links, unexpected runtime dependencies and linked
static archive members. Source, bounded source changes, notices, compiler command,
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
8 tests passed. The initial red run failed because the builder did not yet exist.
Tests cover source checksum rejection, exact dynamic linkage, rejection of static
archives and extra payload libraries, symlink boundaries, patch context checks,
and launcher paths/arguments containing spaces.

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

| Package | Version | SHA-256 |
| --- | --- | --- |
| libfuse3-dev | 3.14.0-5build1 | `cfa8460d23b5143d14bb336b3008784df72a5e9fc32057ec1dc85beadca9aecb` |
| libsquashfuse0 | 0.5.0-2build1 | `19594e096e5772165df1ecb4382f63c1b267cf14703c15ee02011b503d0715b3` |
| libsquashfuse-dev | 0.5.0-2build1 | `7ba23670f18cfea18955ba98d03d630045da82f8a5f9506e68b0411441df8124` |
| squashfs-tools | 1:4.6.1-1build1 | `87fae263846bab255d4a51ad9fc623685497ad830db60758dde39589c9fdadcb` |

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

The **not yet run** integrated-candidate command, after the tree is ready and the
build slot/budget are available, is:

```sh
python3 scripts/build_release.py --bundles deb,appimage \
  --appimage-sysroot "$thin_task/root"
```

The wrapper supplies those isolated tool/library paths only to the thin builder
and its artifact verifier. A separately built fresh deb can also be passed to
`scripts/build_thin_appimage.py --deb PATH --output PATH.AppImage --sysroot PATH`,
with the extracted `usr/bin` on `PATH`; inspecting/running the result on this host
also needs that temporary `LD_LIBRARY_PATH`. These commands are reproduction
instructions, not evidence that the full application image has been built.

## Scope and remaining checks

This is a host-dependent package for a compatible Linux x86_64 system, tested on
Ubuntu 24.04 with squashfuse 0.5.0 and FUSE 3. Recipients still need the deb's
GTK/WebKit dependencies, `libsquashfuse0`, `libfuse3-3` and `fuse3`. The extraction
fallback avoids mounting; it still needs those host libraries. Universal Linux
portability, code signing and public distribution are not claimed. Build receipts
provide inspectable evidence, not protection against deliberate forgery.

The integrated application build, actual candidate gate inspection, no-display
startup and installation/use validation remain in progress under renewed authorization.
The source builder and a minimal runtime probe do not establish those outcomes.

## Review round

The independent release reviewer verified the source/notice pins, exact source
patch output, actual runtime hash, four dynamic dependencies and compiler link
map; the 8 builder tests and 16 inventory/policy tests passed in that review.
The license reviewer requested the host LGPL notice/copy described above; the
resulting 9 approved source/license/notice files have matching content hashes.
The runtime bytes are unchanged by that notice addition.

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
inputs were reacquired and checked against the hashes above. They are currently
retained in the same task-local directory for the coordinated Linux build and
validation; final cleanup must wait until all consuming agents have finished.
