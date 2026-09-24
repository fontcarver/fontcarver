I reviewed the branch against the brief/dispatch, ran the suite in debug and release, and ran independent preservation probes (temporary test files, since deleted; worktree `git status` is clean).

## Spec Compliance — ❌

**Verified present and correct**
- ✅ API surface matches the brief exactly: `ExportFormat/ExportOptions/ExportWarning/ExportReport`, `export`, `FwSource`, `compile_static`, `preserve_merge`, `wrap_woff1`, `shaping_fingerprint` (+ `wrap_woff2`, `touched_since_import`). Defaults match (`keep_tt_instructions/preserve_untouched_tables = true`).
- ✅ `cargo test -p fw-compile` 10/10 pass (debug), release 6/6 pass, `clippy --all-targets -D warnings` and `fmt --check` clean. DejaVu release export **531 ms** (< 5 s). Timing assertions gated on `cfg!(not(debug_assertions))`.
- ✅ `DSIG` never copied (`preserve.rs:33`, `:70`); verified absent from output.
- ✅ `FontBuilder` seeding: no duplicate tags, directory sorted, and `head.checksumAdjustment` is correctly recomputed (I recomputed it independently: `0xd02cb3eb` both).
- ✅ Raw non-layout tables (`FFTM MATH kern`) and `GSUB/GPOS/GDEF` byte-identical on the untouched DejaVu path; `fpgm/prep/cvt ` present and byte-identical; `A` has 194 instruction bytes; harfrust fingerprints identical for "Hello fi AVWA", "ffi ffl", "To VA Wa"; `hmtx` advances identical for all 6253 glyphs; `post` v2 names 6253/6253 identical; OS/2 fsType, panose, unicode/codepage ranges, vendor, weight/width, selection, family class, sub/super/strikeout all preserved; `head.flags`, `lowestRecPPEM`, `unitsPerEm`, `macStyle`, `fontRevision` preserved; `hhea` ascender/descender/lineGap/caret preserved; `maxp` TT limits preserved.
- ✅ Warning codes are stable `&'static str`: `remove-overlap-unavailable`, `autohint-unavailable`, `hinting-dropped`, `layout-dropped-glyph-order-changed`, `provenance-hash-mismatch`, `post-names-omitted-capacity`.
- ✅ Glyph-order-change probe (remove "V"): `GSUB/GPOS/GDEF/BASE` dropped with `layout-dropped-glyph-order-changed`.
- ✅ Scratch font + `Master.kerning_h` → GPOS present and the −50 kern is applied in shaping.
- ✅ WOFF1 validated by a **second decoder** (fontTools 4.63): reads as `flavor=woff`, 20 tables, every table byte-identical to the input sfnt; my structural check confirms sorted tags, contiguous 4-aligned offsets, `compLength ≤ origLength`, header `length` == file size, no trailing bytes. WOFF2 round-trips through `wuff` and fontTools (`A`'s 194-byte program survives).
- ✅ No `unwrap`/`expect`/`panic!`/slice indexing anywhere in `src/` (only two safe widening casts). Files 24–640 lines; `#![warn(missing_docs)]` with clippy `-D warnings` proves every pub item is documented.

**Not met** — the core preservation contract (spec §1.5 "hinting of untouched glyphs") is not actually satisfied; details below.

## Strengths
- Genuinely panic-free on font data, with a well-shaped error enum and `Error::parse/write` helpers — unusual discipline for this much binary I/O.
- The hand-written WOFF1 encoder is correct and independently verifiable; the WOFF2 C++ toolchain requirement is documented in `lib.rs:3`.
- Clean module split, no per-glyph re-parsing (the font is parsed once per phase), rayon left to fontc, budgets met with a 10× margin.
- `provenance_order` re-hashes `original_bytes` and disables provenance-dependent preservation on mismatch — a nice safety property most implementations skip.

## Issues

#### Critical

**C1. Preserved TrueType instructions are re-attached to re-ordered outlines → silent hinting corruption on every untouched font.** `crates/fw-compile/src/preserve.rs:238-274` copies each model glyph's `tt_instructions` onto fontc's *recompiled* `glyf` entry. TrueType instructions address points by index, but fontc's outlines are not the original ones:
- **0 of DejaVu's 3583 simple glyphs** keep the original point order (0 identical, 0 pure rotations, 3583 different); 153 glyphs also change point *count* (e.g. gid 8: 52 → 47 points, 137 instruction bytes re-attached anyway). Same for NotoSans (379 point-count changes, 2616 glyphs given stale instructions).
- fontTools confirms glyph `A`: original coords `[(700,1294),(426,551),(975,551),…]`, export `[(975,551),(426,551),(700,1294),…]` — the contour is reversed/rotated — with byte-identical 194-byte programs.
- Root cause: `fw-model/src/contour.rs:201` (`build_closed` rotates each contour to its first on-curve point) plus write-fonts re-encoding (implied on-curve points dropped). The model *does* carry the original index per point (`Point.tt_index`, set in `fw-sfnt/src/read/glyf.rs:132`) and `fw-compile` ignores it.
- The result is worse than dropping hinting: the exported font runs the original bytecode against a different point numbering, so hinted rasterization is arbitrary. The test only asserts "instructions are non-empty" and shaping equality (which is hinting-independent), so it passes.
- The dispatch anticipated exactly this and authorized the safe path: *"if re-injection is impractical, keep the original `glyf`+`loca` verbatim when no glyph is touched and no glyph was added/removed, and document that."* That fallback (or a `tt_index`-based verification that bails out with `hinting-dropped`) is the fix.

**C2. Instruction re-attachment maps compiled glyph IDs to model order positionally.** `preserve.rs:238` zips `0..glyph_count` with `font.glyphs.values().filter(export)`. fontir *moves* `.notdef` to gid 0 (`fontir-1.0.0/src/glyph.rs:679`), so any model whose `.notdef` is not first gets instructions attached to the wrong glyphs. Probe with model order `[A, .notdef, B]` and distinct instruction streams: gid 0 `.notdef` received A's bytecode, gid 1 `A` received `.notdef`'s — no warning. The count check at `preserve.rs:226` cannot catch a reorder. `large_font::cmap_format_12` (`large_font.rs:36-48`) makes the same positional assumption, so a >65k-glyph font with a non-leading `.notdef` gets a wrong cmap.

**C3. A user glyph is silently renamed to `.notdef` when the model has none.** `source.rs:67-78` renames the first exported glyph to `.notdef`. Probe: a font of `A (U+0041)` and `B (U+0042)` exports as `[.notdef, B]` — A's outline becomes `.notdef`, **U+0041 maps to nothing**, and `report.warnings` is empty. This is exactly the "no silent data loss" prohibition, and it is also unnecessary: fontir already synthesizes a proper `.notdef` and forces it to gid 0 (`ensure_notdef_exists_and_is_gid_0`). Delete the block.

#### Important

**I1. Glyph-ID-indexed raw tables are copied verbatim after the glyph set changes.** `preserve.rs:68-84` gates only `GSUB/GPOS/GDEF/BASE` on glyph order. After removing "V" from DejaVu, the output still contains the original `kern` **byte-identical** and `MATH` — both now reference shifted gids (verified). `fw-sfnt`'s `RAW_ONLY` list also passes through `hdmx VDMX LTSH EBDT/EBLC CBDT/CBLC sbix COLR/CPAL SVG VORG gvar HVAR MVAR cvar`; copying `gvar`/`cvar`/`HVAR`/`MVAR` into a static export is invalid regardless of order (`fvar` is regenerated away). These should be dropped with a warning when `verified_order != Some(true)`.

**I2. Localized and non-Windows `name` records are collapsed and mislabeled.** `metadata.rs:12-21` keys every record with `NameKey::new(id, text)`, which hard-codes platform 3 / lang 0x409 and ignores `NameRecord.language` and `.platform`. Probe with name ID 0 in `en` and `de`: the output has **one** record, lang 0x409, text `"(c) Deutsch"` — the English string is gone and the German one is labeled US English. Same for ID 16 (`"Typo FR"` wins). DejaVu loses all 13 Macintosh-platform records (26 → 13). `NameKey::new_with_lang` exists and should be used with a BCP-47 → Windows LCID mapping; at minimum, don't let one language silently overwrite another.

**I3. Layout preservation silently turns off when provenance bytes are absent, with a wrong warning.** `preserve.rs:93-107` returns `None` if `extra.source.original_bytes` is `None` — which is the normal state after a `.fwf` save/reopen (`original_bytes` is `#[serde(skip)]`, `fw-model/src/extra.rs:27`). Probe: with `original_bytes = None`, `GSUB/GPOS/GDEF` are all dropped and the only warning is `layout-dropped-glyph-order-changed`, which is false — the order did not change. Meanwhile hinting *is* still reattached in that case (`verified_order != Some(false)`), so the two rules disagree about how much they trust missing provenance. Needs a distinct code (e.g. `layout-dropped-provenance-unavailable`) and a consistent policy.

**I4. cmap format 14 (Unicode variation sequences) is dropped silently.** `Glyph.variation_selectors` (`fw-model/src/glyph.rs:44`), which `fw-sfnt` populates, is never read; `source.rs:443-447` only forwards `unicodes`. `cmap` is a CONSUMED table so the original can't come back via `raw_tables`. This matters for the CJK-first-class requirement and deserves at least a warning.

**I5. The large-font workaround loses all glyph names, is mis-thresholded, weakens `cmap`, and is undocumented where it matters.** Verifying check #3:
- *Necessary?* Partly. `write-fonts` `Post::new_v2` does `(258 + storage.len()).try_into().unwrap()` — a genuine **panic** once a font has more than 65,277 distinct non-standard names. So a fallback is needed at the very top end.
- *Correctly gated?* No. `LARGE_GLYPH_COUNT = 65_000` (`large_font.rs:11`) fires ~277 glyphs before the real limit, so fonts in 65,001..65,277 lose all names for nothing.
- *Does it keep names another way?* No. `post_format_3` (`large_font.rs:97-108`) discards every glyph name, and the original `post` is not in `extra.raw_tables` (it's CONSUMED by the reader), so nothing recovers them. For Source Han Sans SC the export loses all 65,535 names. Even at the limit, ~65,277 names *are* representable.
- *Collateral:* codepoints are suppressed for every glyph (`source.rs:141`, `:443`) and the replacement cmap emits **only format 12** on (0,4)/(3,10) — no format 4 BMP subtable, which some consumers still require. (The subtable bytes themselves are well-formed; I checked the header arithmetic.)
- *Documented/tested?* The `postscript_names`-to-258-standard-names hack at `source.rs:215-228` (whose only purpose is to dodge the upstream panic, and which yields exactly one glyph of headroom) carries **no comment at all**. The only coverage is `roundtrip_shaping.rs:22`, which needs a 65k-glyph external fixture and returns early in debug — so CI without that font never exercises this path.

#### Minor
- `head.created`/`modified` are regenerated on every export (two exports of the same document differ in those bytes), even though the model has `open_type_head_created` and `MiscMetadata::created` exists — `metadata.rs:128-176` never sets it. Non-reproducible output.
- `subset_tables` `CORE` (`export.rs:183-186`) omits `fpgm/prep/cvt `, so any `table_subset` yields a font whose glyph programs call undefined functions.
- `preserve.rs:52`: touching any glyph emits `hinting-dropped` even for a font with no hinting at all (verified).
- Numeric clamping happens without warnings: coordinates 90000 → 32767 and advance 70000 → 65535 came through silently. No panic (good), but the brief asked for warnings.
- `preserve_untouched_tables: false` drops `GSUB/GPOS/GDEF/kern/MATH` with **zero** warnings.
- `gasp` is re-emitted as version 1 (source was 0); OS/2 is upgraded v1 → v4 with `usMaxContext = 0` despite ligatures.
- `feature_source` (`source.rs:497-509`) rebuilds class definitions by string-trimming `[`/`]` — fragile for real FEA class bodies.
- Out of scope but visible here: the export gains 37 bogus cmap entries (U+0000, U+0008, U+0080…) because `fw-sfnt/src/read/cmap.rs:36` merges the Macintosh (1,0) subtable's byte values as Unicode codepoints. Shaping is unaffected; the round trip is not byte-clean.

## Assessment

**Task quality: Needs fixes.** The scaffolding, WOFF path, error discipline, and performance are solid, but the task's headline guarantee is not delivered: preserved TrueType instructions are attached to re-ordered outlines (C1) and to the wrong glyphs whenever `.notdef` isn't first (C2), both silently, and a font without `.notdef` loses a glyph (C3) — plus name-record and gid-indexed-table losses that the tests don't cover.