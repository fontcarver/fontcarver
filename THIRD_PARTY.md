# Third-party notices

FontWorkshop itself is licensed `MIT OR Apache-2.0` (see `LICENSE-MIT`,
`LICENSE-APACHE` and `docs/licensing.md`). This file records everything in the
repository that comes from somewhere else: adapted source code, test-font
fixtures, and the licenses of the dependency graph.

---

## 1. FontForge

FontForge is FontWorkshop's behavioural and format reference. The `fontforge/`
directory is a local research checkout: it is listed in `.gitignore`, is never
committed, is never linked, embedded or shipped, and is not part of any build.
The research written from it lives in `docs/research/*.md`.

FontForge as a whole is distributed under the GPLv3-or-later, but per
`fontforge/LICENSE` the large majority of its source — written by George Williams
between 2000 and 2012 — is under the 3-clause ("revised") BSD license, and each
file's own header is authoritative. FontWorkshop may adapt code only from
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
endorses FontWorkshop; see the trademark note in `docs/licensing.md` §4.

### 1.2 FontWorkshop files that adapt FontForge code

| FontWorkshop file | FontForge source file | License in that file's header |
| --- | --- | --- |
| _(none)_ | — | — |

**As of 2026-09-04 no FontWorkshop source file contains code adapted from
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

- `crates/fw-geom/src/edit.rs:14` — `SimplifyOptions` documents the FontForge-
  compatible flag set (ignore slopes / ignore extrema / smooth curves / force
  lines / nearly-hv lines / start at extrema) around `kurbo::simplify::simplify_bezpath`.
- `crates/fw-geom/src/edit.rs:66` — `simplify()` describes those flags; the
  fitting core is kurbo's.
- `crates/fw-geom/src/edit.rs:213` — `add_extrema()` notes that its 0.01-unit
  endpoint tolerance matches FontForge's tolerant `ForceEndPointExtrema` policy.
  Naming the policy is description; the extrema are computed with
  `kurbo::ParamCurveExtrema`.
- `crates/fw-fixtures/src/lib.rs:26,34` — the `"fontforge-tests"` key resolves to
  `<workspace>/fontforge/tests/fonts` in the (gitignored, uncommitted) reference
  checkout, so tests can opt into a broader font corpus when it happens to be
  present locally. It is a path lookup, returning `None` when absent; nothing is
  copied or shipped.

---

## 2. Test-font fixtures

Fixtures are data aggregated with FontWorkshop, not part of it; their licenses do
not affect FontWorkshop's own license. Sources and hashes are in
`fixtures/README.md`. The license column below was verified from each font's own
`name` table (name ID 13 "license description" / 14 "license URL", falling back to
name ID 0 "copyright") rather than taken on trust.

### 2.1 Committed — `fixtures/fonts/`

| File | License (as verified) | Evidence |
| --- | --- | --- |
| `DejaVuSans.ttf` | Bitstream Vera Fonts license (permissive, MIT-style) + public-domain DejaVu changes; Arev additions © Tavmjong Bah | name ID 0/13/14: "Copyright (c) 2003 by Bitstream, Inc. … DejaVu changes are in public domain", <http://dejavu.sourceforge.net/wiki/index.php/License> |
| `DejaVuSans.woff` | same as above (WOFF wrapper of the committed TTF) | identical `name` table |
| `NotoSans-Regular.ttf` | SIL Open Font License 1.1 | name ID 13 "licensed under the SIL Open Font License, Version 1.1", name ID 14 <http://scripts.sil.org/OFL> |
| `NotoSerifTibetan-Regular.ttf` | SIL Open Font License 1.1 | name ID 13/14, same wording |
| `CantarellMin.woff2` | **GNU GPL v3 or later** — see flag below | name ID 13: "This font is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License … either version 3 of the License, or (at your option) any later version"; name ID 0 "Copyright (c) 2009 Dave Crossland" |
| `test1012.otf` | **Undeclared in the font; inherits GPL-3.0-or-later as FontForge test data** — see flag below | name ID 0 is only "Copyright (c) 2019,"; no name ID 13/14. Provenance per `fixtures/README.md`: FontForge `tests/fonts/test1012.otf` |
| `NimbusRoman-Regular-o.otf` | **URW++ Base35: Aladdin Free Public License / GPL** — see flag below | name ID 0 "(URW)++,Copyright 2014 by (URW)++ Design & Development"; no name ID 13/14. Provenance per `fixtures/README.md`: Debian `fonts-urw-base35` |

`fixtures/ufo/Minimal.ufo` is a hand-written 3-glyph UFO created for this project
and is covered by FontWorkshop's own license.

**Flag — three committed fixtures are not permissively licensed.** They are font
*data* in aggregation, so they do not change the license of FontWorkshop's code,
but this repository does redistribute them:

1. `CantarellMin.woff2` — `fixtures/README.md` records it as "SIL Open Font
   License 1.1 (Cantarell)", but this is the **2009 Dave Crossland release, whose
   embedded license is GPLv3-or-later**; Cantarell was only relicensed to the OFL
   later. The README row is wrong and should be corrected or the fixture replaced
   with an OFL-era Cantarell.
2. `test1012.otf` — a FontForge test font with no embedded license, so it takes
   FontForge's overall GPL-3.0-or-later. `fixtures/README.md` already labels it
   that way. It is used by `crates/fw-sfnt/tests/read_otf.rs`,
   `crates/fw-sfnt/tests/robustness.rs` and `crates/fw-api/tests/dispatch.rs`.
3. `NimbusRoman-Regular-o.otf` — URW++ Base35, distributed under the Aladdin Free
   Public License and the GPL; neither is on the allowed list in
   `docs/licensing.md` §5.

None of the three is shipped in a FontWorkshop build artifact — they are only test
inputs under `fixtures/` — and nothing is derived from them. Recommended follow-up
(not done here, since this change is licensing metadata only): replace all three
with OFL or public-domain equivalents, and fix the `CantarellMin.woff2` row in
`fixtures/README.md`.

### 2.2 Optional, not committed

`fw_fixtures::external` resolves two fixtures only if they exist on the local
machine, and dependent tests skip when they do not:

- `source-han-sans-sc-otf` → Source Han Sans SC Regular, **SIL Open Font License
  1.1** (Adobe). Not in this repository.
- `fontforge-tests` → `<workspace>/fontforge/tests/fonts` from the gitignored
  FontForge reference checkout, GPL-3.0-or-later as part of FontForge. Not in this
  repository, never shipped.

---

## 3. Dependency licenses

### 3.1 Rust crates

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
distributing a Larger Work under other terms. FontWorkshop uses all five
unmodified, as ordinary crates.io dependencies, and modifies none of their files,
so nothing is triggered beyond keeping their source available (which crates.io
does) and this notice. **Ruled acceptable.** All five are transitive, none is
chosen directly by FontWorkshop:

| Crate | Version | Why it is in the graph |
| --- | --- | --- |
| `cssparser` | 0.36.0 | `dom_query` → `tauri-utils` → `tauri` / `tauri-build` / `wry` |
| `cssparser-macros` | 0.6.1 | via `cssparser` |
| `dtoa-short` | 0.3.5 | via `cssparser` |
| `selectors` | 0.36.1 | `dom_query` → `tauri-utils` → `tauri` / `tauri-build` / `wry` |
| `option-ext` | 0.2.0 | `dirs-sys` → `dirs` → `fw-api`, and `dirs` → `tauri` / `tauri-build` / `tray-icon` / `wry` |

#### LGPL mention

`r-efi` 5.3.0 and 6.0.0 are `MIT OR Apache-2.0 OR LGPL-2.1-or-later`. FontWorkshop
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
workspace (production and development): **257 packages**.

| License | Packages |
| --- | ---: |
| MIT | 208 |
| Apache-2.0 | 19 |
| ISC | 11 |
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
