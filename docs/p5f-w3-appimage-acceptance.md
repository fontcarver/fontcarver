# Native AppImage inventory and license gate — 2026-10-02

Task: [p5f-w3-appimage](superpowers/plans/tasks/p5f-w3-appimage.md).
This work separates reproducible inspection from approval of a maintained build
recipe. It does not approve the historical Tauri AppImage.

## Historical artifact: actual inspection

The inspected pre-wave artifact is
`/home/chen/.cache/fontcarver-release-target/release/bundle/appimage/FontCarver_0.1.0_amd64.AppImage`,
95,799,800 bytes, SHA-256
`13e9560e0c466b0b56264f037538bb36f1c730f6766a4e619a17b78a060cab63`.
Its own runtime reports `AppImage/type2-runtime` commit `8f39b89` and offset 944632. That prefix's SHA-256 is
`e60b68e5c46f416f74aacbe6aeabca1f681a73c85089f5e6236af974e0e99566`.

The trusted retained image was extracted under this task's scratch directory.
The diagnostic command does not execute the image:

```sh
python3 scripts/appimage_inventory.py \
  --payload .scratch/p5f-w3-appimage/extracted/squashfs-root \
  --appimage /home/chen/.cache/fontcarver-release-target/release/bundle/appimage/FontCarver_0.1.0_amd64.AppImage \
  --runtime-bytes 944632 \
  --output .scratch/p5f-w3-appimage/inventory.json
```

Result: **2,197 paths, 1,475 regular files, 35 symlinks, 177 shared objects**.
All objects have current-host package candidates. The 129 candidates include
75 packages whose machine-readable copyright labels mention copyleft and
30 with no such labels; ten copyright documents ship inside the image.
The original 126/73 survey and this 129/75 rerun use different candidate
resolution; neither count is a confirmed list of copyleft binaries. Some
basenames are owned by more than one package. Missing labels remain unknown.
Every current-host candidate includes its package/source version, file hash,
architecture match and whether its bytes exactly match the image's file.

The complete [JSON evidence](acceptance/p5f-release/appimage-inventory-2026-10-02.json)
retains all path hashes and explicitly says `release_eligible: false`.
The 308 MiB extraction was removed after preserving evidence.

## Source obligations and concrete alternatives

The [exact runtime license](https://github.com/AppImage/type2-runtime/blob/8f39b89/LICENSE)
lists statically linked LGPL libfuse. Removing GTK/WebKit from the payload while
keeping this prebuilt runtime would still violate the repository's shipped-code
policy. LGPL source/notice/relink conditions are separate from that project
policy; copying license texts alone is insufficient. The relevant text is
[LGPL 2.1 §§4–6, distributed by libfuse](https://github.com/libfuse/libfuse/blob/master/LGPL2.txt).

The existing Debian strategy uses host GTK/WebKit and copies no native library
closure into its payload. A thin AppImage can use the same strategy only if its
runtime also uses host libraries. Legacy AppImageKit 13 was investigated:
its [runtime linkage](https://github.com/AppImage/AppImageKit/blob/13/src/build-runtime.cmake)
and FUSE `dlopen` mechanism offer a possible route, but merely downloading that
older executable would not establish the complete binary's provenance. The
parallel thin-builder task instead pursues a pinned current MIT runtime source
with dynamic host FUSE3/squashfuse, retaining source, license and linker evidence.

## Gate and focused verification

`appimage_policy.require_releasable_appimage(payload, artifact)` requires the
audited thin builder's pinned source/license files, exact launcher, one payload
ELF (FontCarver), no `.so` libraries or other executable files, safe symlinks,
the actual runtime prefix hash and SquashFS offset, the exact approved dynamic
host-library dependencies, and a linker map without static FUSE/squashfuse.
No environment override or receipt `release_eligible` field grants approval.
This verifies a reviewed build recipe's artifact evidence; it does not claim
that arbitrary binaries or deliberately forged build logs can be licensed by
examining their names.

```text
TMPDIR=$PWD/.scratch/p5f-w3-appimage/tmp PYTHONDONTWRITEBYTECODE=1 \
  python3 -m unittest discover -s scripts -p test_appimage_inventory.py
Ran 18 tests
OK

TMPDIR=$PWD/.scratch/p5f-w3-appimage/tmp PYTHONDONTWRITEBYTECODE=1 \
  python3 -m unittest discover -s scripts -p test_check_release_licenses.py
Ran 10 tests
OK
```

Tests cover missing/changed source provenance, runtime hashes, absent dynamic
dependencies, static FUSE linkage, extra binaries/libraries, launcher changes,
malformed receipts, library search-path overrides, other static archives,
symlink escapes and missing license labels. The first
test-first run failed because the inventory implementation was absent; the
implemented suite passes.

The real historical artifact was also run through the integrated bundle
inspector after the gate was wired in:

```text
python3 scripts/check_release_bundle.py \
  --bundle-dir /home/chen/.cache/fontcarver-release-target/release/bundle \
  --expected appimage \
  --output .scratch/p5f-w3-appimage/old-appimage-rejected.json
FAIL: .scratch/p5f-w3-appimage/old-appimage-rejected.json
AppImage release is blocked: missing audited thin-runtime BUILD.json
exit status: 1
```

This is the intended rejection, not a missing test. Its failure manifest has
`ok: false`; extraction scratch was automatically removed.

The common release notice bundle now also includes a Linux host-library use and
replacement notice, with unchanged LGPL 2/2.1 text from GTK/libfuse upstream.
The exact source URLs and SHA-256 values are retained in
`scripts/release_licenses/native/HOST-LIBRARIES.txt`; the generator verifies the
license-text hashes before reading package metadata. This ships documentation,
not additional LGPL code, and does not change the source-package allow-list.

## Independent review

The release reviewer found no inventory blocker. Review of the artifact gate
required explicit validation of malformed receipt shapes, parity with builder
RPATH/RUNPATH and static-archive restrictions, and binding every retained input
and linker map to its hash. These are implemented and regression-tested. The
reviewer independently accepted the corrected gate and checked the real built
runtime's source pins/linker map. Whole-image positive evidence remains a
separate check, below.

## Integration and remaining evidence

**Tooling and negative artifact acceptance are complete; row 62 remains Partial.**
The later user instruction was to finish the current repair and stop, so a fresh
integrated installer build/install was not started for this task. The audited
thin runtime compiled and its actual dynamic dependencies were checked; this
does not replace positive verification of a complete FontCarver AppImage.

The next packaging task must regenerate release notices, build the integrated
deb and thin AppImage, run the artifact gate against that real image, and verify
installation/launch with its documented host dependencies. The parent owns
that TODO and the final row-62 recommendation. Native macOS/Windows evidence,
signing and publication are not established by this report.

No FontForge code, new Cargo/npm dependency, translation or desktop automation
was involved.

Final combined focused check: **28 tests passed** (18 inventory/policy and ten
source-license tests); both retained host-license texts matched their upstream
SHA-256 pins, and the edited Markdown passed Prettier. All task-owned processes
had exited and `.scratch/p5f-w3-appimage/` was removed. The final artifact check
passed: build **16.16 GiB / 10,821 files**, shared temporary **0.05 GiB / 1,852
files**, free **605.64 GiB**. The build warning above 10 GiB remains below the
20 GiB ceiling; other agents own the remaining temporary work. Git status was
reviewed, with only this task's intended files included in its report.
