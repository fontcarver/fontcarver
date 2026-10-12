# AppImage bundled-library license census — 2026-10-08

What the gate in `scripts/appimage_policy.py` actually reports for a stock Tauri
AppImage payload, and what remains to be decided. This is the measurement the
spec's §10 called unscopeable; it is larger than the "~30 packages" figure that was
quoted from the inventory summary before the gate existed.

## Method

The classifier in `scripts/appimage_licenses.py` was run over the recorded payload
inventory of the first real AppImage build,
`docs/acceptance/p5f-release/appimage-inventory-2026-10-02.json` (177 shared
objects, 130 owning-package candidates, captured on Ubuntu 24.04). No new build was
needed: the inventory already records each payload file with its owning package and
the `License:` labels of that package's Debian copyright file.

Caveat carried from the inventory itself: host package ownership is a hint about
the inspection host, **not verified build provenance**, and Debian labels describe
source packages. A label is a review lead, not a per-binary ruling.

## Result

| Verdict | Packages | Meaning |
| --- | --- | --- |
| `pass` | 40 | permissive or LGPL only; ship with the §6 notice |
| `ruling-required` | 57 | a GPL label appears somewhere in the copyright file |
| `unidentified` | 30 | no machine-readable `License:` field at all |
| `forbidden` | 3 | AGPL-3 |

**90 of 130 packages need a human determination before a stock AppImage can ship.**

## The three forbidden packages are genuinely in the payload

`libwmf-0.2-7`, `libwmf-0.2-7-gtk` and `libwmflite-0.2-7` all carry
`AGPL-3 with Font exception`. Their files in the payload are:

```
usr/lib/libwmf-0.2.so.7
usr/lib/libwmflite-0.2.so.7
usr/lib/x86_64-linux-gnu/gdk-pixbuf-2.0/2.10.0/loaders/io-wmf.so
```

**This cannot be ruled around** — `docs/licensing.md` §5 forbids AGPL in a native
payload, AGPL has no linking exception, and we would be redistributing the library
itself. But the fix is cheap, and an earlier draft of this section got that wrong by
calling for payload surgery.

They arrive as a gdk-pixbuf image loader for Windows Metafile. The bundler copied
**all 14** gdk-pixbuf loaders present on the build machine — `io-wmf.so` alongside
the png, svg, tiff and webp ones — not because FontCarver needs WMF but because the
loader directory was there. `libwmf-0.2.so.7` and `libwmflite-0.2.so.7` follow only
because `io-wmf.so` links them; nothing else in the payload does.

So this is **build-environment hygiene, not payload surgery**: removing the
`libwmf-0.2-7-gtk` package from the build host drops the loader, and the two
libraries follow it out because nothing is left to pull them in. `apt-get -s remove
libwmf-0.2-7-gtk` on the capture host removes that package alone and breaks nothing.

That also bounds how much of this census is inherent. The inventory was captured on
a developer workstation (Ubuntu 24.04.4), whose package set is broader than a
curated CI runner's. Whether `libwmf-0.2-7-gtk` exists on GitHub's `ubuntu-24.04`
image is unverified here, so the AGPL row may not appear in a CI build at all.

## The 57 ruling-required packages include the core of the stack

Among them: `libgtk-3-0t64`, `libwebkit2gtk-4.1-0`, `libjavascriptcoregtk-4.1-0`,
`libglib2.0-0t64`, `libgstreamer1.0-0`, `libicu74`, `libsystemd0`, `libpng16-16t64`.

Their GPL labels are overwhelmingly for build tooling, test suites and documentation
(GFDL) rather than the shipped `.so` — libgtk's `GPL-3+` covers its build scripts,
not the library. That is exactly why the classifier demands a recorded ruling rather
than refusing outright. But each ruling is a determination someone signs, recorded
in `appimage_licenses.RULINGS` with its reason.

Two entries are `-dev` packages (`libglib2.0-dev`, `libgdk-pixbuf-2.0-dev`), matched
because the bundler copied unversioned `.so` symlink targets. A runtime payload
should not contain development files at all; that is a bundler over-copy worth
removing regardless of licensing.

## The 30 unidentified packages mostly cannot be resolved by tooling

Re-running the label extraction against this host's current copyright files:

| | Count |
| --- | --- |
| now carry labels (resolved) | 3 — `libpixman-1-0` (Expat), `libxdamage1` (Xorg), `libxdmcp6` (OpenGroup-MIT) |
| still no `License:` field | 25 |
| not installed on this host, undeterminable here | 2 — `libcanberra-gtk3-0t64`, `libcanberra0t64` |

The 25 use pre-DEP-5 free-text copyright files. They are readable — most are X.org
MIT-style permission grants, with krb5 MIT, cairo LGPL/MPL, avahi and libtasn1 LGPL,
orc BSD — but the determination comes from reading prose, not from a field.

**Seven of the 30 copyright files changed in the six days between the inventory
capture and this census.** The determination set is therefore not stable: it moves
with the build host's package updates, so rulings go stale and the census has to be
re-run per release.

## What this means

Every row in this census is a function of **what is installed on the build
machine**, not of what FontCarver needs — the bundler sweeps what it finds. The
strongest lever is therefore a minimal, declared build environment rather than 90
individual determinations: a smaller host package set yields a smaller payload and a
smaller census. The AGPL row is the clearest case, removable with one package.

What remains after that is still substantial, and it recurs as the build host
drifts. That is a per-release treadmill rather than a one-time cost, and it is the
empirical version of the judgement the repository's original blanket ban made for
weaker stated reasons (see `docs/licensing.md` §5.1).

It does not reopen the policy: LGPL in a payload remains permitted with notices, and
that part works. What the census shows is how much of a real GTK/WebKit payload
falls outside the easy case.

Flatpak is the comparison worth making, because its GNOME runtime supplies GTK and
WebKit from Flathub rather than from our artifact; nothing in that set would be
redistributed by this project, so none of these 90 determinations would be ours.
That trade is recorded here as a fact, not a decision.
