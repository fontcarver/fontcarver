# Native AppImage inventory and license gate — 2026-10-02

Task: [p5f-w3-appimage](superpowers/plans/tasks/p5f-w3-appimage.md).
**Complete for the reviewed Linux host-library recipe.** The final candidate
built from `f6f6cf9` passes independent whole-image inventory, license/payload
policy and actual input-deb comparison, including its corrected AppImage format
marker. The first thin candidate and historical library-bundling Tauri image
remain rejected. This does not claim signing, publication or other native
platform acceptance.

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
parallel thin-builder task instead builds a pinned current MIT runtime source
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
Ran 22 tests
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
runtime's source pins/linker map. Whole-image positive evidence is recorded below.

## Superseded integrated candidate: independent inspection

The resumed local packaging run produced
`target/release/bundle/appimage/FontCarver_0.1.0_amd64.AppImage`, **23,675,448
bytes**, SHA-256
`713f93d87d859ecf652b7e517e3b0daeaac454e9c5dd1cb2536054c984ec5d00`.
Its [historical complete inventory](acceptance/p5f-release/thin-appimage-inventory-before-marker-fix-2026-10-02.json)
records **1,818 paths, 1,160 regular files, no symlinks, one ELF executable and
zero bundled shared objects**. All regular files have SHA-256 hashes.

The retained [historical build manifest](acceptance/p5f-w3-packaging/round-1-6e35f70/release-manifest.json)
records revision `6e35f70fb530e71365c93f1d67ac8ff57b73b745` and 1,998 build inputs
with fingerprint
`22102bdf1b4f9dc417958f2e2998737925d159bcb15849a4fda32363aeb2cb56`.
The embedded receipt names the actual input deb hash
`66542dd9a317ccc2f9bd23f30bd91077198073ae7258ea0732243d6f5e83011e`.
Independent extraction and hashing confirmed that the AppImage's application
binary is identical to that deb's binary, SHA-256
`d96b55bc3691aab3201899e6914f82e8fe942210d30321fca0009e2dfe7df370`.

The real package passed this independently executed command (the temporary
dependency root contained the thin-builder task's verified distribution packages):

```sh
TMPDIR="$PWD/.scratch/p5f-w3-appimage/tmp" \
LD_LIBRARY_PATH="$PWD/.scratch/p5f-w3-thin-appimage/root/usr/lib/x86_64-linux-gnu" \
PYTHONDONTWRITEBYTECODE=1 \
python3 scripts/check_release_bundle.py \
  --bundle-dir target/release/bundle --expected appimage \
  --output .scratch/p5f-w3-appimage/positive-manifest.json
```

Result: **PASS**, `ok: true`. The inspector verified 630 source-package entries
and 1,126 retained notice files, plus the common host-library notice and both
LGPL text hashes. The artifact policy accepted all **nine source/license pins**.
The 49,720-byte runtime prefix has SHA-256
`7c017539a65934c5ffbe66a805c630323608794748d2a82cd5d88ef321c18f91` and exactly
four dynamic dependencies: `libc.so.6`, `libfuse3.so.3`, `libsquashfuse.so.0` and
`libsquashfuse_ll.so.0`. It has no runtime library search-path override or linked
static archive members. A second direct policy check over the separately
extracted complete image also passed.

This establishes the reviewed local recipe and artifact's agreement, including
its actual binary and notice hashes. It is not a signature, a compiler
reproducibility proof or a way to approve deliberately forged linker receipts.
The [thin-builder acceptance](p5f-w3-thin-appimage-acceptance.md) separately covers
FUSE and extract-and-run execution; the [packaging acceptance](p5f-w3-packaging-acceptance.md)
covers the actual deb installation. This is a host-dependent Linux candidate,
not a self-contained distribution for arbitrary Linux installations.

That first candidate's license/payload result is retained as historical evidence.
The corrected candidate below must also pass the bundle-format check. The parent
owns the overall row-62 recommendation; native macOS/Windows evidence, signing
and publication are outside this local pass.

## Bundle-type marker correction

Independent review found the real thin candidate still contained the unique
`__TAURI_BUNDLE_TYPE_VAR_DEB` inherited from its input deb. Tauri-utils 2.9.3
`src/platform.rs` reads this marker as the package format. The builder now changes
exactly three bytes, `DEB` to `APP`, in the extracted copy, leaving the deb intact.
It rejects missing, duplicate, conflicting or unexpected markers and records the
before/after ELF hashes and the changed-byte offset in `BUILD.json.application`.

The artifact gate independently requires one actual `APP` marker, verifies its
offset/output hash, reverses only those three bytes in memory and compares the
result with the recorded input ELF hash. Four new regression methods initially
reproduced 14 incorrectly accepted subcases; after the fix all 32 focused tests
pass. The existing real pre-fix candidate now fails with
`AppImage application must have the APP bundle-type marker`. This is a recorded
metadata correction, not a claim that the former file is the final accepted
candidate. Runtime code and all nine source/license pins are unchanged.

## Final corrected candidate: independent acceptance

The final image at
`target/release/bundle/appimage/FontCarver_0.1.0_amd64.AppImage` is **23,683,640
bytes**, SHA-256
`3a9029b1aa83d39d78365b18202b39aa5a035f640083dfb4e53151c205793536`.
Its input deb is **24,235,798 bytes**, SHA-256
`e01096f3abd387e5758dc177cfe5a85e4ec5030613d1f3f1c890aac789e429de`.
The final [build manifest](acceptance/p5f-w3-packaging/release-manifest.json)
records revision `f6f6cf9ce481d11314e25c540b5719fd0951a00f`, **2,008** build
inputs and fingerprint
`2fe41daf9aa27d48230fb3ac970ab119867c49245f2dcdff47017c60c93a6624`.

The bundle-inspection command above was rerun independently against this image:
**PASS**, `ok: true`. A separate extraction, direct artifact-policy call and
complete hash inventory also passed. The [final JSON evidence](acceptance/p5f-release/thin-appimage-inventory-2026-10-02.json)
records **1,818 paths, 1,160 regular files, zero symlinks, one application ELF
and zero bundled shared objects**. It retains every file hash, all nine accepted
source/license pins, 630 source-package entries, 1,126 notice files and the three
verified host-library notice resources. The runtime prefix and its four dynamic
dependencies are unchanged from the reviewed recipe.

Independent extraction of the actual input deb verified the receipt's whole-deb
hash. Its **66,983,608-byte** application ELF and the image's ELF are identical
outside the three bytes at offset **4,743,271**: the deb contains the unique
`DEB` marker and the AppImage contains the unique `APP` marker. The deb ELF hash
is `9715a2ad24ecaa9975cea03a56cf68ffe2de95931a569884b900000980fa976e`;
the image ELF hash is
`c8fe34032a54e4e609ae0e641f0c149becc67dbef66c2086d1059187066f9432`.
Reversing those three bytes reproduces the actual deb ELF hash. All other
**1,144 regular files** from the deb have identical hashes in the AppImage.

These results approve this reviewed recipe and actual local candidate within
the stated provenance boundary. The source fingerprint, linker map and receipt
are recorded build evidence, not signatures or proof against a malicious
compiler or forged logs. Host libraries remain system prerequisites and are not
redistributed. The parent owns overall acceptance; native macOS/Windows work,
signing and publication remain outside this local task.

No FontForge code, new Cargo/npm dependency, translation or desktop automation
was involved.

The final focused check is **32 tests passed** (22 inventory/policy and ten
source-license tests); both retained host-license texts matched their upstream
SHA-256 pins. Beyond acceptance evidence, the resumed code change is confined to
the actual bundle-type defect and its gate regressions.

After retaining the final inventory, this task removed its own **146,769,460
bytes** of temporary extractions and logs. It left the thin builder's dependency
root and other tasks' files under their owners' control. Final resource check:

```text
python3 scripts/check_artifacts.py
build: 5.33 GiB, 12,262 files
temporary: 0.14 GiB, 1,994 files
workspace free: 616.25 GiB
PASS
```
