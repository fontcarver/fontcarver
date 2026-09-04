# License change report — MIT OR Apache-2.0

Date: 2026-09-04 · Branch: `p1-outline-editing` · Repo: `/home/chen/githubprojects/pixelfonts/fontworkshop`

FontWorkshop was relicensed from the `GPL-3.0-or-later` placeholder to the Rust
convention **`MIT OR Apache-2.0`** (dual, licensee's choice). Copyright line:
`Copyright (c) 2026 Tom Chen and the FontWorkshop contributors`.

## 1. Files changed

### Modified

| File | Change |
| --- | --- |
| `/home/chen/githubprojects/pixelfonts/fontworkshop/Cargo.toml` | `[workspace.package] license` → `"MIT OR Apache-2.0"` (was `"GPL-3.0-or-later"`) |
| `/home/chen/githubprojects/pixelfonts/fontworkshop/package.json` | added `"license": "MIT OR Apache-2.0"` |
| `/home/chen/githubprojects/pixelfonts/fontworkshop/app/package.json` | added `"license": "MIT OR Apache-2.0"` |
| `/home/chen/githubprojects/pixelfonts/fontworkshop/CONVENTIONS.md` | new section **Licensing and porting from FontForge** (5 rules); extended the Rust dependency rule with the permissive-license requirement |

### Created

| File | Contents |
| --- | --- |
| `/home/chen/githubprojects/pixelfonts/fontworkshop/LICENSE-MIT` | standard MIT text with the copyright line above |
| `/home/chen/githubprojects/pixelfonts/fontworkshop/LICENSE-APACHE` | full Apache-2.0 text including the appendix, filled in for 2026 Tom Chen and the FontWorkshop contributors |
| `/home/chen/githubprojects/pixelfonts/fontworkshop/README.md` | project description, status, layout, doc pointers, build steps, **License** section (dual + contribution clause + FontForge relationship + trademark disclaimer) |
| `/home/chen/githubprojects/pixelfonts/fontworkshop/docs/licensing.md` | full reasoning: idea/expression split, what `fontforge/LICENSE` actually says, the BSD-only adaptation rule, GPL-only re-implementation rule, no-linking rule, the checkout is never shipped, trademark note, dependency policy (allowed / forbidden / MPL case-by-case), fixture policy |
| `/home/chen/githubprojects/pixelfonts/fontworkshop/THIRD_PARTY.md` | verbatim FontForge revised-BSD text + copyright-notice form, the adaptation table, behavioural-reference list, fixture licenses verified from `name` tables, Rust + npm dependency license surveys, re-run commands |

No crate manifest needed editing: all 12 workspace members
(`crates/fw-api`, `fw-cli`, `fw-compile`, `fw-devserver`, `fw-doc`, `fw-fixtures`,
`fw-formats`, `fw-geom`, `fw-glyphdata`, `fw-model`, `fw-sfnt`, and `src-tauri`)
already carry `license.workspace = true` on line 5 of their `Cargo.toml`.

A repo-wide grep for `GPL` across `*.toml`, `*.json`, `*.yml`, `*.rs`, `*.ts`,
`*.tsx` (excluding `fontforge/`, `target/`, `node_modules/`, `.worktrees/`,
`Cargo.lock`) now returns nothing.

## 2. FontForge adaptation table

**Empty — no FontWorkshop source file contains code adapted from FontForge.**

Method: `grep -rn -i "fontforge" crates/ app/src src-tauri` returned five hits in
two files; each was read in context. A second scan for FontForge identifiers
(`SplineChar`, `SplineFont`, `SplineSet`, `SplinePoint`, `StemInfo`, `BasePoint`,
`RefChar`, `BDFChar`, `FVMenu`, `CVMenu`, `ff_`, `FF_`) and for porting phrases
(`adapted from`, `ported from`, `derived from`, `.c` filenames, `George Williams`,
`revised BSD`) across all 152 `.rs` files plus `app/src` returned no FontForge
code. `crates/fw-formats/src/` has no SFD module yet, so no FontForge-format
parser exists to have been ported.

| Category | File:line | Verdict |
| --- | --- | --- |
| (i) adapted code | — | **none** |
| (ii) behavioural reference | `crates/fw-geom/src/edit.rs:14` | `SimplifyOptions` doc comment names the FontForge-compatible flag set; the core is `kurbo::simplify::simplify_bezpath` |
| (ii) behavioural reference | `crates/fw-geom/src/edit.rs:66` | `simplify()` doc comment: "kurbo's curve fitter and FontForge-style flags" |
| (ii) behavioural reference | `crates/fw-geom/src/edit.rs:213` | `add_extrema()` doc comment: 0.01-unit endpoint tolerance "matching FontForge's tolerant `ForceEndPointExtrema` policy"; extrema come from `kurbo::ParamCurveExtrema` |
| (iii) unrelated mention | `crates/fw-fixtures/src/lib.rs:26,34` | the `"fontforge-tests"` external-fixture key resolves to `<workspace>/fontforge/tests/fonts`; a path lookup returning `None` when the gitignored checkout is absent |

Because there are no adapted blocks, task step 7 (add the
`// Adapted from FontForge …` attribution comment to each) had nothing to do. The
comment template and the `THIRD_PARTY.md` §1.2 table are in place for the first
adaptation that lands.

**No adapted code traces to a GPL-only FontForge file** — there is no adapted code
at all.

## 3. Dependency license summary

### 3.1 Rust — `cargo metadata --format-version 1`

596 external packages reachable from the 12 workspace members, all dependency
kinds (normal + build + dev). **No package is missing a `license` field.**

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

Copyleft mentions, all resolved:

- **MPL-2.0 × 5** — `cssparser` 0.36.0, `cssparser-macros` 0.6.1, `dtoa-short`
  0.3.5, `selectors` 0.36.1 (all via `dom_query` → `tauri-utils` → Tauri/wry) and
  `option-ext` 0.2.0 (via `dirs-sys` → `dirs`, used by `fw-api` and Tauri). MPL-2.0
  is file-level copyleft and §3.3 permits a differently licensed Larger Work; all
  five are unmodified transitive dependencies. **Ruled acceptable and recorded in
  `THIRD_PARTY.md` §3.1** as `docs/licensing.md` §5 requires.
- **`r-efi` 5.3.0 / 6.0.0** — `MIT OR Apache-2.0 OR LGPL-2.1-or-later`; the
  permissive branch is taken. Not a concern.
- **No GPL, AGPL, SSPL, CDDL or EUPL package anywhere in the graph.**

36 packages use the legacy `MIT/Apache-2.0` spelling (and a few `/`, ` / `, `AND`
variants); these are dual MIT+Apache-2.0 and are treated as `MIT OR Apache-2.0`.

### 3.2 npm — `pnpm --dir app licenses list --json`

257 packages in the `app` workspace (prod + dev). Tool present (pnpm 10.32.1);
survey ran clean.

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

**No copyleft and no unknown/missing license.** Two noted explicitly:
`caniuse-lite` 1.0.30001810 (CC-BY-4.0 — a build-time browser-support dataset,
attribution-only, not copyleft) and `minimatch` 10.2.6 (BlueOak-1.0.0 —
OSI-approved permissive).

## 4. Flagged items

### 4.1 Three committed font fixtures are not permissively licensed — **action recommended**

Licenses were verified from each font's own `name` table (IDs 0/13/14) with
fontTools 4.63.0, not taken from `fixtures/README.md`. Two rows in that README
turned out to be wrong or incomplete:

1. **`fixtures/fonts/CantarellMin.woff2` — README says "SIL Open Font License 1.1
   (Cantarell)"; the font itself says GPLv3-or-later.** Name ID 13 reads: *"This
   font is free software: you can redistribute it and/or modify it under the terms
   of the GNU General Public License as published by the Free Software Foundation,
   either version 3 of the License, or (at your option) any later version."*, with
   name ID 0 `Copyright (c) 2009 Dave Crossland <dave@lab6.com>`. This is the 2009
   Cantarell, from before the OFL relicensing. **The `fixtures/README.md` row is
   factually wrong and should be corrected**; better, replace the fixture with an
   OFL-era Cantarell. Used by `crates/fw-sfnt/tests/read_containers.rs:41`,
   `read_otf.rs:110`, `robustness.rs:12`.
2. **`fixtures/fonts/test1012.otf` — GPL-3.0-or-later.** The font declares no
   license (name ID 0 is only `Copyright (c) 2019,`); it is FontForge test data and
   so inherits FontForge's overall GPLv3-or-later. `fixtures/README.md` already
   labels it correctly. Used by `crates/fw-sfnt/tests/read_otf.rs:8`,
   `robustness.rs:10`, `crates/fw-api/tests/dispatch.rs:396`.
3. **`fixtures/fonts/NimbusRoman-Regular-o.otf` — Aladdin Free Public License /
   GPL** (URW++ Base35 via Debian `fonts-urw-base35`); `fixtures/README.md` says so.
   Neither license is on the allowed list. Used by
   `crates/fw-sfnt/tests/read_otf.rs:54,113`, `robustness.rs:11`.

Assessment: these are font **data** in aggregation, not linked into or derived
from FontWorkshop's code, and none is shipped in a build artifact — they are test
inputs under `fixtures/`. FontWorkshop's own `MIT OR Apache-2.0` license is
therefore unaffected. But this repository does redistribute all three, so a
release tarball or a repo clone carries GPL-licensed data. Recommended follow-up,
**not done in this change** (which was scoped to licensing metadata and docs):
replace all three fixtures with OFL or public-domain equivalents, and fix the
`CantarellMin.woff2` row in `fixtures/README.md`. All three are recorded with the
verified license and a flag in `THIRD_PARTY.md` §2.1.

The other four committed fixtures are clean: `DejaVuSans.ttf` / `.woff` (Bitstream
Vera license, permissive MIT-style, plus public-domain DejaVu changes — confirmed
name IDs 0/13/14), `NotoSans-Regular.ttf` and `NotoSerifTibetan-Regular.ttf` (SIL
OFL 1.1 — confirmed name IDs 13/14). `fixtures/ufo/Minimal.ufo` is hand-written
for this project.

### 4.2 MPL-2.0 dependencies — resolved, not blocking

Five transitive MPL-2.0 crates (§3.1). Ruled acceptable and recorded, which is
exactly what `docs/licensing.md` §5 requires for MPL-2.0. Nothing to fix.

### 4.3 Pre-existing, unrelated: stale generated TS bindings

`pnpm --dir app check` failed on first run with
`src/views/fontinfo/NamesTable.tsx(18,71): error TS2339: Property 'Other' does not
exist on type 'never'` — `app/src/api/generated/NamePlatform.ts` was stale and had
lost the `{ "Other": number }` variant. Unrelated to this change (only
`package.json` metadata was touched). Fixed by running `pnpm --dir app gen:api`;
the generated directory is gitignored, so nothing was committed. Worth knowing:
the generated bindings on this working tree can drift from `fw-api`.

### 4.4 `cargo-deny` is not wired into CI

The dependency policy in `docs/licensing.md` §5 is currently enforced by review
only. Noted in `THIRD_PARTY.md` §3.3 as a suggested follow-up: encode the
allow-list in `deny.toml` and add a CI job.

## 5. Verification

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | pass (exit 0, no output) |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass (exit 0, `Finished dev profile`) |
| `cargo test -p fw-geom` | pass (all suites ok, 0 failed) |
| `pnpm --dir app check` | pass (exit 0) after `pnpm --dir app gen:api` regenerated the stale bindings (§4.3) |

## 6. Commit

Committed on `p1-outline-editing` (not pushed):

```
chore(license): relicense as MIT OR Apache-2.0 with FontForge attribution policy
```
