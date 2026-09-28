# Third-party notices

FontCarver itself is licensed `MIT OR Apache-2.0` (see `LICENSE-MIT`,
`LICENSE-APACHE` and `docs/licensing.md`). This file records everything in the
repository that comes from somewhere else: adapted source code, test-font
fixtures, and the licenses of the dependency graph.

---

## 1. FontForge

FontForge is FontCarver's behavioural and format reference. The `fontforge/`
directory is a local research checkout: it is listed in `.gitignore`, is never
committed, is never linked, embedded or shipped, and is not part of any build.
The research written from it lives in `docs/research/*.md`.

FontForge as a whole is distributed under the GPLv3-or-later, but per
`fontforge/LICENSE` the large majority of its source — written by George Williams
between 2000 and 2012 — is under the 3-clause ("revised") BSD license, and each
file's own header is authoritative. FontCarver may adapt code only from
revised-BSD files, with attribution; GPL-only files are re-implemented from
specifications or observed behaviour without copying. See `docs/licensing.md` §3.

### 1.1 The revised BSD license used by FontForge

Reproduced verbatim from `fontforge/LICENSE`:

```
Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions
are met:

Redistributions of source code must retain the above copyright
notice, this list of conditions and the following disclaimer.

Redistributions in binary form must reproduce the above copyright
notice, this list of conditions and the following disclaimer in the
documentation and/or other materials provided with the distribution.

The name of the author may not be used to endorse or promote
products derived from this software without specific prior written
permission.

THIS SOFTWARE IS PROVIDED BY THE AUTHOR "AS IS" AND ANY EXPRESS
OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE
GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER
IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN
IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

The copyright notice this license refers to is carried per file in the FontForge
sources, in the form:

```
Copyright (C) 2000-2012 by George Williams
```

with later contributors appended where applicable (for example
`fontforge/fontforge/splineutil2.c`:
`Copyright (C) 2000-2012 by George Williams, 2021 by Linus Romer`). Any adaptation
listed in §1.2 reproduces the exact notice of the file it came from. Per the
license's third clause, neither George Williams nor any other FontForge author
endorses FontCarver; see the trademark note in `docs/licensing.md` §4.

### 1.2 FontCarver files that adapt FontForge code

| FontCarver file | FontForge source file | License in that file's header |
| --- | --- | --- |
| _(none)_ | — | — |

**As of 2026-09-04 no FontCarver source file contains code adapted from
FontForge.** The repository was surveyed with
`grep -rn -i "fontforge" crates/ app/src src-tauri` and with a scan for FontForge
identifiers (`SplineChar`, `SplineFont`, `SplineSet`, `SplinePoint`, `StemInfo`,
`RefChar`, `BDFChar`, …); no matches beyond the behavioural references in §1.3.
When the first adaptation lands, it goes in the table above and carries the
`// Adapted from FontForge …` comment required by `CONVENTIONS.md`.

### 1.3 Behavioural references only (no code adapted)

These files name FontForge in comments to say which behaviour they match. They
contain no FontForge code; the implementations are written on `kurbo` and the rest
of the Rust font stack.

- `crates/fc-geom/src/edit.rs:14` — `SimplifyOptions` documents the FontForge-
  compatible flag set (ignore slopes / ignore extrema / smooth curves / force
  lines / nearly-hv lines / start at extrema) around `kurbo::simplify::simplify_bezpath`.
- `crates/fc-geom/src/edit.rs:66` — `simplify()` describes those flags; the
  fitting core is kurbo's.
- `crates/fc-geom/src/edit.rs:213` — `add_extrema()` notes that its 0.01-unit
  endpoint tolerance matches FontForge's tolerant `ForceEndPointExtrema` policy.
  Naming the policy is description; the extrema are computed with
  `kurbo::ParamCurveExtrema`.
- `crates/fc-fixtures/src/lib.rs:26,34` — the `"fontforge-tests"` key resolves to
  `<workspace>/fontforge/tests/fonts` in the (gitignored, uncommitted) reference
  checkout, so tests can opt into a broader font corpus when it happens to be
  present locally. It is a path lookup, returning `None` when absent; nothing is
  copied or shipped.

---

## 2. Test-font fixtures

Fixtures are data aggregated with FontCarver, not part of it; their licenses do
not affect FontCarver's own license. Sources and hashes are in
`fixtures/README.md`. The license column below was verified from each font's own
`name` table (name ID 13 "license description" / 14 "license URL", falling back to
name ID 0 "copyright") rather than taken on trust.

### 2.1 Committed — `fixtures/fonts/`

| File | License (as verified) | Evidence |
| --- | --- | --- |
| `DejaVuSans.ttf` | Bitstream Vera Fonts license (permissive, MIT-style) + public-domain DejaVu changes; Arev additions © Tavmjong Bah | name ID 0/13/14: "Copyright (c) 2003 by Bitstream, Inc. … DejaVu changes are in public domain", <http://dejavu.sourceforge.net/wiki/index.php/License> |
| `DejaVuSans.woff` | same as above (WOFF wrapper of the committed TTF) | identical `name` table |
| `DejaVuSansMin.woff2` | same as above (53-glyph WOFF2 subset of the committed TTF) | name ID 0/13/14 retained unchanged from `DejaVuSans.ttf` |
| `NotoSans-Regular.ttf` | SIL Open Font License 1.1 | name ID 13 "licensed under the SIL Open Font License, Version 1.1", name ID 14 <http://scripts.sil.org/OFL> |
| `NotoSerifTibetan-Regular.ttf` | SIL Open Font License 1.1 | name ID 13/14, same wording |
| `SourceSans3Min.otf` | SIL Open Font License 1.1 | name ID 13 identifies OFL 1.1 and name ID 14 is <http://scripts.sil.org/OFL>; name ID 0 identifies Adobe and Reserved Font Name `Source`; the modified subset's primary family is `FontWorkshop CFF Sans Fixture` |
| `SourceSerif4Min.otf` | SIL Open Font License 1.1 | name ID 13 identifies OFL 1.1 and name ID 14 is <http://scripts.sil.org/OFL>; name ID 0 identifies Adobe and Reserved Font Name `Source`; the modified subset's primary family is `FontWorkshop CFF Serif Fixture` |

`fixtures/ufo/Minimal.ufo` is a hand-written 3-glyph UFO created for this project
and is covered by FontCarver's own license.

All committed font fixtures are now under licenses allowed by
`docs/licensing.md` §5: SIL OFL 1.1 or the Bitstream Vera/DejaVu license. The full
OFL 1.1 text and the Adobe fonts' per-font copyright and Reserved Font Name
statements are in `fixtures/LICENSE-OFL.txt`. Exact source URLs, versions, subset
commands, sizes, and hashes are in `fixtures/README.md`.

### 2.2 Optional, not committed

`fc_fixtures::external` resolves two fixtures only if they exist on the local
machine, and dependent tests skip when they do not:

- `source-han-sans-sc-otf` → Source Han Sans SC Regular, **SIL Open Font License
  1.1** (Adobe). Not in this repository.
- `fontforge-tests` → `<workspace>/fontforge/tests/fonts` from the gitignored
  FontForge reference checkout, GPL-3.0-or-later as part of FontForge. Not in this
  repository, never shipped.

---

## 2A. Website fonts and specimens — `site/`

The website at `fontcarver.com` self-hosts its type rather than loading it from
a font service, so these files are redistributed with the site. All three are
**SIL Open Font License 1.1**; the full OFL text is in
`fixtures/LICENSE-OFL.txt`.

| File in `site/assets/fonts/` | Family | Copyright | License evidence | SHA-256 (first 16) |
|---|---|---|---|---|
| `source-serif-4-variable.woff2` | Source Serif 4 (variable, `wght` 200–900, `opsz` 8–60) | Adobe, Reserved Font Name `Source` | OFL 1.1 as published by Adobe at <https://github.com/adobe-fonts/source-serif> | `f9d47f68388a6eef` |
| `ibm-plex-mono-400.woff2` | IBM Plex Mono Regular | IBM Corp. | OFL 1.1 as published at <https://github.com/IBM/plex> | `61f330a0921f52d1` |
| `ibm-plex-mono-500.woff2` | IBM Plex Mono Medium | IBM Corp. | OFL 1.1 as published at <https://github.com/IBM/plex> | `b9414e01257b2426` |

Each file is a Latin subset produced with `fontTools.subset` from the
corresponding `@fontsource` release; the OFL permits subsetting and
redistribution under the same license, and the files keep their original name
table entries for copyright (name ID 0), license (13) and license URL (14).

### Glyph outlines in `site/assets/js/specimens.js`

The hero figure draws two real glyphs. Their outline coordinates are extracted
by `site/tools/extract-outlines.py` and committed as data:

| Glyph | Source font | License |
|---|---|---|
| `g` (U+0067) | Source Serif 4 Regular, from the subset above | SIL OFL 1.1 (Adobe, RFN `Source`) |
| `永` (U+6C38) | Noto Serif CJK SC Regular | SIL OFL 1.1 (Adobe/Google, RFN `Source`, `Noto`) |

Noto Serif CJK is **not** redistributed: only the coordinates of one glyph are
committed, and the font itself is read from the developer's machine when the
tool runs. Both attributions appear in the website's footer.

---

## 3. Dependency licenses

P2a eligibility (2026-09-16) adds a direct `fc-doc` dependency on the already
locked `fea-rs` 1.0.0 parser, shared with `fontc`, to inspect feature ownership.
Its installed Cargo manifest declares `MIT/Apache-2.0`; no package version or
external package is added to the workspace lockfile by this dependency edge.

### 3.1 Rust crates

P2c/P2d (2026-09-19) reuses already locked permissive dependencies; no new
third-party package or version is introduced. `fc-formats` now directly uses
`glyphs-reader` 1.0.0 (`MIT/Apache-2.0`) for native Glyphs parsing and `read-fonts`
0.43.3 (`MIT OR Apache-2.0`, including its AGL feature) for native Type 1 parsing
and glyph-name mapping. `tempfile` 3.27.0 (`MIT OR Apache-2.0`) moves from test
to runtime use in `fc-formats` and `fc-api` for atomic export replacement.
`harfrust` 0.13.3 (`MIT`) is a direct `fc-formats` test dependency for independent
SFD shaping checks. These licenses were checked in the installed Cargo manifests.
The internal `fc-compile` → `fc-sfnt` edge now supports CFF output at runtime.
No FontForge implementation code or additional font fixtures are redistributed.

P2b-01 adds only a test dependency from `fc-formats` to the existing `fc-compile`
workspace crate (`MIT OR Apache-2.0`) to verify real FEA compilation and shaping
after UFO round trips. No third-party package or locked version is added.

Surveyed 2026-09-04 with `cargo metadata --format-version 1`, over the fully
resolved graph reachable from the 12 workspace members, all dependency kinds
included (normal, build and dev): **596 external packages**.

| License expression | Packages |
| --- | ---: |
| MIT OR Apache-2.0 | 284 |
| MIT | 141 |
| Apache-2.0 OR MIT | 39 |
| MIT/Apache-2.0 | 36 |
| Unicode-3.0 | 18 |
| Zlib OR Apache-2.0 OR MIT | 18 |
| Unlicense OR MIT | 10 |
| Apache-2.0 | 7 |
| Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 5 |
| MPL-2.0 | 5 |
| Apache-2.0/MIT | 3 |
| BSD-2-Clause OR Apache-2.0 OR MIT | 2 |
| BSD-3-Clause | 2 |
| BSD-3-Clause AND MIT | 2 |
| BSD-3-Clause OR MIT OR Apache-2.0 | 2 |
| BSD-3-Clause/MIT | 2 |
| MIT OR Apache-2.0 OR LGPL-2.1-or-later | 2 |
| MIT OR Apache-2.0 OR Zlib | 2 |
| MIT OR Zlib OR Apache-2.0 | 2 |
| Unlicense/MIT | 2 |
| Zlib | 2 |
| (MIT OR Apache-2.0) AND Unicode-3.0 | 1 |
| 0BSD OR MIT OR Apache-2.0 | 1 |
| Apache-2.0 / MIT | 1 |
| Apache-2.0 AND MIT | 1 |
| Apache-2.0 OR BSL-1.0 | 1 |
| Apache-2.0 WITH LLVM-exception | 1 |
| BSD-2-Clause | 1 |
| CC0-1.0 OR MIT-0 OR Apache-2.0 | 1 |
| ISC | 1 |
| MIT AND BSD-3-Clause | 1 |

No package is missing a `license` field. Every entry is on the allowed list in
`docs/licensing.md` §5, except for the copyleft mentions below.

#### MPL-2.0 ruling

Five crates are MPL-2.0. MPL-2.0 is **file-level** copyleft: its obligations
attach to the MPL-licensed files themselves, and §3.3 explicitly permits
distributing a Larger Work under other terms. FontCarver uses all five
unmodified, as ordinary crates.io dependencies, and modifies none of their files,
so nothing is triggered beyond keeping their source available (which crates.io
does) and this notice. **Ruled acceptable.** All five are transitive, none is
chosen directly by FontCarver:

| Crate | Version | Why it is in the graph |
| --- | --- | --- |
| `cssparser` | 0.36.0 | `dom_query` → `tauri-utils` → `tauri` / `tauri-build` / `wry` |
| `cssparser-macros` | 0.6.1 | via `cssparser` |
| `dtoa-short` | 0.3.5 | via `cssparser` |
| `selectors` | 0.36.1 | `dom_query` → `tauri-utils` → `tauri` / `tauri-build` / `wry` |
| `option-ext` | 0.2.0 | `dirs-sys` → `dirs` → `fc-api`, and `dirs` → `tauri` / `tauri-build` / `tray-icon` / `wry` |

#### LGPL mention

`r-efi` 5.3.0 and 6.0.0 are `MIT OR Apache-2.0 OR LGPL-2.1-or-later`. FontCarver
takes the MIT/Apache-2.0 branch; the LGPL branch is not exercised. **Not a
concern.** (`r-efi` is a UEFI target-support crate pulled in by the standard
library's dependency set; it is not linked on Linux/macOS/Windows builds.)

No GPL, AGPL, SSPL, CDDL or EUPL crate appears anywhere in the graph.

#### Non-SPDX-normalised expressions

36 packages declare `MIT/Apache-2.0` and a few others use `/` or ` / ` separators
or `AND` where `OR` was meant. These are legacy Cargo spellings of dual MIT and
Apache-2.0 licensing and are treated as `MIT OR Apache-2.0`.

### 3.2 npm packages

Surveyed 2026-09-04 with `pnpm --dir app licenses list --json` over the `app`
workspace (production and development): **258 packages**.

| License | Packages |
| --- | ---: |
| MIT | 208 |
| Apache-2.0 | 19 |
| ISC | 12 |
| BSD-2-Clause | 8 |
| Apache-2.0 OR MIT | 3 |
| BSD-3-Clause | 3 |
| MIT OR Apache-2.0 | 2 |
| BlueOak-1.0.0 | 1 |
| CC-BY-4.0 | 1 |
| MIT-0 | 1 |

**No copyleft package (GPL / LGPL / AGPL / MPL / SSPL / CDDL / EUPL) and no
package with an unknown or missing license.** Two entries are worth naming
explicitly:

- `caniuse-lite` 1.0.30001810 — **CC-BY-4.0**. A build-time browser-support
  *dataset* used by `browserslist`/Vite; permissive, attribution-only, and not
  copyleft. It contributes data, not shipped code. Acceptable.
- `minimatch` 10.2.6 — **BlueOak-1.0.0**, an OSI-approved permissive license with
  no copyleft obligation. Acceptable.

#### P2b CodeMirror editor additions — 2026-09-17

The Features editor adds the following exact pinned packages. CodeMirror supplies
accessible source editing, completion, syntax highlighting and located diagnostics;
`@lezer/highlight` maps syntax tokens to the existing light/dark design tokens.
The installed package manifests and `pnpm --dir app licenses list --json` verify
**MIT** for all 13 newly locked packages (including the six transitive packages).
Existing dependency versions are unchanged.

| Direct package | Locked version | License |
| --- | --- | --- |
| `@codemirror/state` | 6.7.5 | MIT |
| `@codemirror/view` | 6.43.12 | MIT |
| `@codemirror/language` | 6.12.4 | MIT |
| `@codemirror/autocomplete` | 6.20.3 | MIT |
| `@codemirror/lint` | 6.9.7 | MIT |
| `@codemirror/commands` | 6.11.1 | MIT |
| `@lezer/highlight` | 1.2.3 | MIT |

New transitive packages (all MIT): `@lezer/common` 1.5.2, `@lezer/lr` 1.4.10,
`@marijn/find-cluster-break` 1.0.4, `crelt` 1.0.7, `style-mod` 4.1.3 and
`w3c-keyname` 2.2.8. No new copyleft dependency is introduced.

### 3.3 Re-running the surveys

Rust — count the license expression of every package reachable from the
workspace members (all dependency kinds):

```sh
cargo metadata --format-version 1 | python3 -c '
import json, sys, collections
m = json.load(sys.stdin)
pkgs = {p["id"]: p for p in m["packages"]}
ws = set(m["workspace_members"])
nodes = {n["id"]: n for n in m["resolve"]["nodes"]}
seen, stack = set(), list(ws)
while stack:
    i = stack.pop()
    if i in seen:
        continue
    seen.add(i)
    stack += [d["pkg"] for d in nodes.get(i, {}).get("deps", [])]
c = collections.Counter(pkgs[i].get("license") or "<NONE>" for i in seen - ws)
print(sum(c.values()), "external packages")
for k, v in c.most_common():
    print(f"{v:5d}  {k}")
'
```

npm:

```sh
pnpm --dir app licenses list
```

`cargo-deny` is not currently wired into CI; if it is added later, this section's
allow-list should be encoded in `deny.toml` so the policy in `docs/licensing.md`
§5 is enforced automatically rather than by review.

### P3 bitmap implementation — 2026-09-22

The bitmap readers/writers, pixel tools and boundary tracer are original
implementations from format specifications; no FontForge source was adapted,
linked, embedded or executed. No external font was added to the repository.
Tests construct their own small bitmap fonts; the optional CJK acceptance input
is read from `FC_CJK_BDF` and remains outside the checkout.

- `tiny-skia 0.11.4` and `tiny-skia-path 0.11.4` (BSD-3-Clause): bounded outline
  coverage rasterization. Added transitive dependencies are `arrayref 0.3.9`
  (BSD-2-Clause) and `strict-num 0.1.1` (MIT).
- Existing `png 0.17.16` (MIT OR Apache-2.0) is now used directly by `fc-sfnt`
  to decode and encode embedded PNG bitmap payloads.
- Existing `flate2` (MIT OR Apache-2.0) is reused by `fc-formats` for compressed
  bitmap files. Existing skrifa/fontations dependencies supply embedded hint
  interpretation, automatic grid fitting and checked bitmap table parsing.

License identifiers were checked in the pinned packages' Cargo manifests.
Specifications: [Adobe BDF](https://adobe-type-tools.github.io/font-tech-notes/pdfs/5005.BDF_Spec.pdf),
[X11 PCF description](https://fontforge.org/docs/techref/pcf-format.html),
[Microsoft FNT KB 65123 archive](https://ftp.zx.net.nz/pub/Patches/ftp.microsoft.com/MISC/KB/en-us/65/123.HTM),
[FONTDIR](https://learn.microsoft.com/en-us/windows/win32/menurc/fontdirentry),
[OpenType EBDT](https://learn.microsoft.com/en-us/typography/opentype/spec/ebdt),
[CBDT](https://learn.microsoft.com/en-us/typography/opentype/spec/cbdt),
[sbix](https://learn.microsoft.com/en-us/typography/opentype/spec/sbix).

### P4 Unicode data and variation implementation — 2026-09-22

- `fc-glyphdata/data/Unihan-17-plane*.tsv` derives from Unicode 17.0.0 Unihan
  (Copyright 2025 Unicode, Inc.; Unicode License v3).
- `JIS0208.TXT` and `KSX1001.TXT` retain Unicode's original permission notices.
- `GB18030-2022-Level1.txt` derives from ICU 77.1 `gb18030-2022.ucm`
  (Unicode License v3; Copyright 2016 and later Unicode, Inc. and others;
  Copyright 2000–2012 International Business Machines Corporation and others).
- Complete Unicode license: `crates/fc-glyphdata/data/UNICODE-LICENSE.txt`.
  Pinned URLs, source hashes, extraction commands and repertoire qualifications:
  `crates/fc-glyphdata/data/README.md`.
- Existing `fontdrasil 1.0.0` (MIT OR Apache-2.0) is used directly by `fc-geom`
  for the same variation model as the fontc compiler. Existing workspace
  `fc-geom` is now used directly by `fc-api` and `fc-compile` for preview and
  compile-time interpolation. No additional third-party variation dependency
  or FontForge source is introduced.

### P5 hinting implementation (2026-09-22)

`fc-hint` reuses the already locked permissive `read-fonts`, `kurbo`, `tempfile`
and `thiserror` packages. TrueType opcode names/decoding come from the public
fontations API. PostScript geometry analysis and the assembly text parser are
original implementations; no FontForge source or executable is used.

The optional `ttfautohint` integration launches a user-installed executable in a
separate process. No ttfautohint or FreeType binary, source or library is added to
the shipped dependency graph. Users obtain and license that tool separately;
see its [official distribution](https://freetype.org/ttfautohint/).

### P5 SVG documents — 2026-09-28

- `roxmltree 0.21.1` (MIT OR Apache-2.0) provides bounded, namespace-aware XML
  parsing with DTD processing disabled, for SVG validation and identifier
  rewriting. The license was checked in the pinned package's Cargo manifest.
- Existing `flate2` (MIT OR Apache-2.0) is reused by `fc-sfnt` for bounded gzip
  SVG document decoding. No FontForge source or new artwork is incorporated.
- The acceptance fixture generated by `scripts/p5_svg_acceptance.py` is original
  geometric artwork, generated outside the repository. The codec follows the
  [OpenType SVG specification](https://learn.microsoft.com/en-us/typography/opentype/spec/svg).
