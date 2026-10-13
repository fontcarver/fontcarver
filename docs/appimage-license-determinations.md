# AppImage bundled-library determinations — hosted runner, 2026-10-13

What the gate in `scripts/appimage_policy.py` blocks in an AppImage built on
GitHub's `ubuntu-24.04` image, read against each package's own copyright file,
with a proposed determination per upstream family.

Evidence: the `AppImage license census` workflow, run on `main`. It builds a real
AppImage, inventories the payload, classifies every owning package and archives
the copyright file of each one. Reproduce it from the Actions tab; the artifact
carries `inventory.json`, `census.json`, `release-manifest.json`, the host's
package list and all 110 copyright files.

**Nothing here is in force.** `appimage_licenses.RULINGS` is still empty and the
gate still refuses the image. These are proposals for a human to accept.

## What changed against the workstation census

| | workstation (2026-10-02) | hosted runner (2026-10-13) |
| --- | --- | --- |
| owning packages | 130 | 110 |
| pass | 40 | 33 |
| ruling-required | 57 | 50 |
| unidentified | 30 | 27 |
| **forbidden (AGPL)** | **3** | **0** |

`docs/appimage-bundled-license-census.md` said the AGPL row might not appear on a
curated runner. It does not: the job prints `no libwmf package on this host`. The
three AGPL packages were a property of the developer workstation, not of
FontCarver, and no payload surgery was needed to remove them.

The gate blocks **124 payload files from 78 owning packages**: 27 with no
machine-readable license field, 50 carrying a GPL label somewhere, and one file
with no owning package at all.

## 1. The one real copyleft library: libjbig

| Package | License | How it arrives |
| --- | --- | --- |
| `libjbig0` | `GPL-2+`, with only `debian/*` beside it | `libpixbufloader-tiff.so` → `libtiff.so.6` → `libjbig.so.0` |

JBIG-KIT is GPL-2+. Its copyright file has no library exception and no second
stanza: the shipped `libjbig.so.0` is GPL-2+, and `docs/licensing.md` §5 forbids
GPL in a native payload.

This repeats the libwmf shape exactly. The bundler copied **all twelve**
gdk-pixbuf image loaders it found — ani, bmp, gif, icns, ico, pnm, qtif, svg,
tga, tiff, xbm, xpm — and only the tiff one pulls libtiff, which pulls libjbig.
A font editor needs none of them except svg.

Unlike libwmf, the loaders cannot be removed by uninstalling a package: they ship
inside `libgdk-pixbuf-2.0-0`, which the application does need.

**Decided: drop them.** `scripts/appimage_trim.py` runs after the bundler, keeps
the SVG loader and removes the rest, then removes the libraries that only those
loaders reached — libtiff and libjbig among them. PNG and JPEG decoding is built
into gdk-pixbuf itself and is unaffected; WebKit decodes page images on its own.
A library nothing in the payload names is left alone, because a module opened by
name at run time has no incoming reference to find. The policy in
`docs/licensing.md` §5 is unchanged, and the image is smaller.

## 2. No machine-readable license field — 27 packages

Each copyright file predates DEP-5 and states its terms in prose. Read, they are:

| Family | Packages | Terms in the file | Proposed |
| --- | --- | --- | --- |
| X.org and XCB | `libxau6` `libxcomposite1` `libxcursor1` `libxdamage1` `libxdmcp6` `libxext6` `libxfixes3` `libxi6` `libxinerama1` `libxkbcommon0` `libxrandr2` `libxrender1` `libxcb-render0` `libxcb-shm0` `libpixman-1-0` | X Consortium / Open Group / MIT permission grants | pass |
| MIT | `libxslt1.1` `libgssapi-krb5-2` `libk5crypto3` `libkrb5-3` `libkrb5support0` | MIT permission grant | pass |
| LGPL | `libavahi-client3` `libavahi-common3` `libtasn1-6` | LGPL | pass, with the §6 notice |
| cairo | `libcairo2` `libcairo-gobject2` | LGPL-2.1 or MPL-1.1, at the recipient's choice | pass, with the §6 notice |
| BSD | `liborc-0.4-0t64` | 2-clause BSD | pass |
| GCC runtime | `libatomic1` | GPL-3 **with the GCC Runtime Library Exception 3.1** | pass — the exception exists precisely to permit this |

## 3. No owning package — 1 file

| File | Origin | Proposed |
| --- | --- | --- |
| `AppRun.wrapped` (31 KB) | The AppImage launcher, installed by linuxdeploy, which the Tauri bundler downloads. [linuxdeploy](https://github.com/linuxdeploy/linuxdeploy/blob/master/LICENSE.txt) and [AppImageKit](https://github.com/AppImage/AppImageKit/blob/master/LICENSE) are both MIT. | pass |

It is not the application: `usr/bin/fontcarver-desktop` is 67 MB and separate.

## 4. A GPL label somewhere in the file — 50 packages

A Debian copyright file describes a whole **source** package. For a library, the
GPL stanzas overwhelmingly cover files an AppImage payload cannot contain. Of the
50:

| Why the GPL label is there | Packages | Proposed |
| --- | --- | --- |
| `debian/*` — the Debian packaging scripts | present in most of the 50 | pass; never in the payload |
| Build scaffolding under an explicit exception — `config.guess`, `config.sub`, `ltmain.sh`, Autoconf and Libtool exceptions | `libffi8` `libharfbuzz-icu0` `libhogweed6t64` `libnettle8t64` `libnghttp2-14` `libp11-kit0` `libunistring5` | pass; the exception permits it and none of it is in the `.so` |
| Tests, docs, code generators | `libglib2.0-0t64` `libgdk-pixbuf-2.0-0` `libgstreamer*` `liblz4-1` `liblzma5` `libsecret-1-0` `libenchant-2-2` and others | pass; not compiled into the shipped object |
| Command-line utilities in the same source package | `libblkid1` `libmount1` (library subtrees are LGPL-2.1+) `libkeyutils1` (`keyutils.*` is LGPL-2+) `libcolord2` (`lib/colord/*` is LGPL-2.1+) `libgnutls30t64` (`GPLv3+` is `src/*`) `libicu74` `libsqlite3-0` | pass; the utilities are not in the payload |
| A disjunctive choice that includes a permissive or lesser option — `X or GPL` | `libatk1.0-0t64` `libatk-bridge2.0-0t64` `libatspi2.0-0t64` (AFL-2.1 or GPL-2+) `libcap2` `libzstd1` (BSD-3 or GPL-2) `libhyphen0` `libpng16-16t64` `libdw1t64` `libelf1t64` (LGPL-3+ or GPL-2+) `libidn2-0` | pass; take the non-GPL option |
| Bison- and flex-generated parsers, under the Bison exception | `libwebkit2gtk-4.1-0` `libjavascriptcoregtk-4.1-0` `libgstreamer1.0-0` `libdw1t64` `libelf1t64` | pass; the exception exists so generated parsers may be used in non-GPL programs |
| Linux kernel UAPI headers | `libsystemd0` `libudev1` (`GPL-2 with Linux-syscall-note exception`) `libevdev2` (`include/linux/input.h`) | pass for the first two on the stated exception; `libevdev2` carries the same kernel headers without the note recorded, and upstream libevdev is MIT |
| macOS-only source | `libenchant-2-2` (`applespell_checker.mm`) | pass; not built on Linux |

Two entries are `-dev` packages (`libglib2.0-dev`, `libgdk-pixbuf-2.0-dev`),
matched because the bundler copied unversioned `.so` symlink targets. A runtime
payload should not carry development files; that is a bundler over-copy worth
removing whatever its license.

`libidn2-0` is the weakest row. Its library sources are `LGPL-3+ or GPL-2+`, but
a later stanza puts `lib/*` under `GPL-3+`, and in DEP-5 the later stanza wins.
Upstream states the library is dual-licensed and the GPL-3+ covers the
command-line tool. Worth confirming against upstream before it is signed.

## What this costs per release

Every row is a function of the build host's package set, and a hosted runner
image is updated continuously. The workstation census found seven of thirty
copyright files changed within six days. These determinations are therefore
valid for **this runner image and this build**, and the census has to be re-run
and re-read per release. That recurring review, not any single library, is the
real cost of publishing an AppImage.

Flatpak remains the comparison worth making: its GNOME runtime supplies GTK and
WebKit from Flathub rather than from our artifact, so none of these 78
determinations would be ours to make. Recorded as a fact, not a decision.
