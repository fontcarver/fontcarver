<!-- Research report generated 2026-09-03 by an Explore agent reading fontforge/ (FontForge git dc05099f, 2026-08-10). Paths are relative to fontforge/. -->

# FontForge Core Data Model — Reference for a Rust/TS Re-implementation

All paths below are under `fontforge/fontforge/`, the FontForge reference checkout in this repository. Note that although the task named `splinefont.h`, roughly half the glyph-level model actually lives in `splinechar.h`, which `splinefont.h:35` includes.

Scalar conventions: `real` = `float` (or `double` under `FONTFORGE_CONFIG_USE_DOUBLE`), `bigreal` = `double`, `extended` = `double` (`splinechar.h:51-57`, `splinefont.h:44`). `BasePoint{real x,y}` (`splinechar.h:59`), `DBounds{minx,maxx,miny,maxy}` (`splinechar.h:66`).

---

## SplineFont — `splinefont.h:1198`

Font-level container. One `SplineFont` per face; MM masters and CID subfonts are themselves `SplineFont`s.

**Names / identity**
- `fontname, fullname, familyname, weight, familyname_with_timestamp, copyright, version`: PostScript-style name fields.
- `filename`: sfd path; `origname`: source font file path; `defbasefilename`, `autosavename`, `chosenname` (for TTC/multi-font files).
- `names` (`struct ttflangname*`, `:955`): per-language array `names[ttf_namemax]` of utf-8 strings, indexed by `enum ttfnames` (`:949`: copyright, family, subfamily, uniqueid, fullname, version, postscriptname, trademark, manufacturer, designer, descriptor, venderurl, designerurl, license, licenseurl, preffamilyname, prefmodifiers, compatfull, sampletext, cidfindfontname, wwsfamily, wwssubfamily). `frommac` bitfield records which came from Mac name records.
- `uniqueid`, `xuid`, `tempuniqueid`, `use_xuid`, `use_uniqueid`, `fondname` (Mac FOND), `macstyle` (`enum style_flags`, `:1391`).

**Metrics**
- `ascent, descent`: em is `ascent+descent` (there is no separate em field). `invalidem` says to use format-specific values on export.
- `italicangle, upos, uwidth`: italic angle, underline position/thickness.
- `ufo_ascent, ufo_descent`: opaque UFO round-trip values (descent negative).
- `pfminfo` (`struct pfminfo`, `:1123`): really the OS/2 + hhea + vhea block — `weight, width, panose[10], stylemap, fstype, linegap, vlinegap, hhead_ascent/descent, os2_typoascent/typodescent/typolinegap, os2_winascent/windescent, sub/sup size+offsets, os2_strikeysize/ypos, os2_capheight, os2_xheight, os2_vendor[4], os2_family_class, codepages[2], unicoderanges[4]`, plus `*_set`/`*_add` flags saying whether each group is explicit or derived.
- `hasvmetrics`: emit vhea/vmtx/VORG.
- `os2_version, use_typo_metrics, weight_width_slope_only, head_optimized_for_cleartype`, `sfntRevision`, `woffMajor/woffMinor/woffMetadata`, `creationtime/modificationtime` (seconds since 1970).
- `design_size, design_range_bottom, design_range_top, fontstyle_id, fontstyle_name` (GPOS `size` feature, tenths of a point).
- `strokewidth`, `strokedfont`, `multilayer` (Type3 with strokes+fills).
- `extrema_bound`, `width_separation`: editor policy values persisted in the font.

**Glyphs & encoding**
- `glyphs` / `glyphcnt` / `glyphmax`: dense array indexed by GID; entries may be NULL. `SplineChar.orig_pos` is the index back into this array.
- `map` (`EncMap*`): the font's default encoding map (the *live* map is on `FontViewBase`).
- `glyphnames` (`struct glyphnamehash*`): name→SplineChar hash cache.
- `uni_interp` (`enum uni_interp`, `:781`): how to interpret the PUA (adobe/greek/japanese/trad-chinese/simp-chinese/korean/ams).
- `for_new_glyphs` (`NameList*`, `:771`): glyph-naming convention.
- `top_enc`, `encodingchanged`, `compacted`.

**Layers**
- `layers` (`LayerInfo*`) / `layer_cnt`: font-wide layer *definitions*; every `SplineChar.layers[]` is parallel to this.
- `grid` (`Layer`, by value): the font-global guideline layer, index `ly_grid == -1`.
- `display_layer`.

**Outline hinting / PostScript**
- `private_dict` (`struct psdict*`, `:90`): raw key/value string pairs of the Type1/CFF Private dict (BlueValues, StdHW, StemSnapH, …). FontForge deliberately keeps it as strings, not parsed fields — see the commented-out `struct psprivate` in `psfont.h:68`.
- `ttf_tables`, `ttf_tab_saved` (`struct ttf_table*`, `:1161`): unparsed table blobs. `fpgm/prep/cvt/maxp` go into `ttf_tables`; user-requested pass-through tables go into `ttf_tab_saved`.
- `cvt_names`: per-cvt-entry comment strings, terminated by the sentinel `END_CVT_NAMES`.
- `gasp_version, gasp_cnt, gasp` (`struct gasp{ppem,flags}`, `:1176`).

**OpenType layout**
- `gsub_lookups, gpos_lookups` (`OTLookup*` lists). Apple `morx` subtables are read into GSUB, Apple `kern` into GPOS.
- `anchor` (`AnchorClass*` list), `kerns`, `vkerns` (`KernClass*` lists), `possub` (`FPST*`), `sm` (`ASM*`), `features` (`MacFeat*`), `feat_names` (`struct otffeatname*`).
- `mark_class_cnt/mark_classes/mark_class_names`: GDEF Mark Attachment Classes (class 0 unused).
- `mark_set_cnt/mark_sets/mark_set_names`: GDEF Mark Filtering Sets (set 0 *is* meaningful).
- `horiz_base, vert_base` (`struct Base`, `:1115` — `baseline_cnt`, `baseline_tags[]`, `scripts` list of `basescript{script, def_baseline, baseline_pos[], langs}`).
- `justify` (`Justify*`, `:650`, JSTF).
- `groups` (`ff_glyphclasses`, `:1181`) and `groupkerns/groupvkerns` (`ff_rawoffsets`, `:1189`): raw UFO groups.plist / kerning.plist round-trip data; `preferred_kerning` selects UFO-native vs feature-file output.

**Other tables**
- `texdata` (`struct texdata`, `:1171`): `type` ∈ {unset,text,math,mathext} plus `params[22]`.
- `MATH` (`struct MATH*`, `:962`): ~55 int16 constants each optionally paired with a `DeviceTable*` adjustment, plus `MinConnectorOverlap`.
- `bitmaps` (`BDFFont*` linked list, sorted by pixelsize).
- `comments`, `fontlog`, `python_persistent` (opaque pickled blob preserved verbatim), `woffMetadata`.

**Multiple-master / CID**
- `mm` (`MMSet*`): non-NULL if this font is part of an MM set.
- `cidregistry, ordering, supplement, cidversion`; `subfontcnt`/`subfonts`; `cidmaster` (back-pointer from a subfont to the top-level CID font).

**Bookkeeping / state**
- `changed`, `changed_since_autosave`, `isnew`, `backedup`, `ticked`, `internal_temp`, `save_to_dir` (sfdir), `compression`, `loadvalidation_state` (`:1076`), `undoes` (`SFUndoes*`).
- UI/cache-only fields exist and should be dropped in a port: `fv`, `metrics`, `fontinfo`, `valwin`, `kcld/vkcld/kcd`, `instr_dlgs`, `cvt_dlg`, `display_size`, `display_antialias`, `display_bbsized`, `desired_row_cnt/col_cnt`, `python_temporary`, various `*warn`/`complained_about_*` bits.

---

## SplineChar — `splinechar.h:578`

- `name`: glyph name (unique within the font).
- `unicodeenc`: primary codepoint, `-1` if none.
- `altuni` (`struct altuni`, `splinefont.h:937`): `{unienc, vs, fid}` list. `vs == -1` means a plain alternate encoding; otherwise `vs` is a Unicode variation selector making this a variant shape.
- `orig_pos`: index into `parent->glyphs`. `ttf_glyph`: output GID (temporary). `lsidebearing`: heavily overloaded temporary (Type1 subr number, otf sidebearing, morx table mask).
- `width, vwidth` (int16), `widthset`.
- `layers` / `layer_cnt`: array of `Layer`. Index 0 = background (`ly_back`), 1 = foreground (`ly_fore`); 2..n are extra background layers in normal fonts and extra *foreground* layers in Type3/multilayer fonts. `layer_cnt` may legitimately be **less** than `parent->layer_cnt` (see `splinefont.c:1919`).
- Hints: `hstem` / `vstem` (`StemInfo*`), `dstem` (`DStemInfo*`, diagonal, TrueType only), `md` (`MinimumDistance*`), `countermask_cnt` / `countermasks` (`HintMask*`), `hconflicts` / `vconflicts`, `manualhints`, `changedsincelasthinted`.
- `ttf_instrs` / `ttf_instrs_len`: raw TrueType bytecode; `instructions_out_of_date`, `numberpointsbackards`, `complained_about_ptnums`.
- `kerns` / `vkerns` (`KernPair*`): **the left glyph of the pair owns the record** and points at the right glyph.
- `possub` (`PST*`): all per-glyph GSUB/GPOS data (positions, pairs, substitutions, ligature definitions, lig carets).
- `ligofme` (`LigList*`, `splinefont.h:520`): derived index — if this glyph is a ligature's *first component*, the list of ligatures it can start. Must be regenerated before save.
- `dependents` (`struct splinecharlist*`): reverse index of glyphs that reference this one.
- `anchor` (`AnchorPoint*` list).
- `glyph_class`: 3 bits; 0 = auto-detect, else `gdef_class + 1` (so 2 → base, 3 → ligature, 4 → mark).
- `color` (`uint32_t`, `COLOR_DEFAULT = 0xfffffffe`), `comment` (utf-8).
- TeX: `tex_height`, `tex_depth` (`TEX_UNDEF = 0x7fff` sentinel).
- MATH: `italic_correction` + `italic_adjusts` (DeviceTable), `top_accent_horiz` + `top_accent_adjusts`, `is_extended_shape`, `vert_variants` / `horiz_variants` (`struct glyphvariants`, `splinefont.h:845`: space-separated `variants` list plus an assembly of `gv_part{component, is_extender, startConnectorLength, endConnectorLength, fullAdvance}`), `mathkern` (`struct mathkern`, `:869`: four `mathkernvertex` corners, each a ladder of `mathkerndata{height,kern}` with device tables; there is one more kern than height).
- `tile_margin` / `tile_bounds`: whitespace when this glyph is used as a fill pattern.
- `glif_name`: UFO file base name; `user_decomp`: user-supplied decomposition.
- `inspiro`: this glyph is edited as spiro curves.
- `parent` (`SplineFont*`); UI/cache: `views`, `charinfo`, `python_sc_object`, `wasopen`, `vs_open`, `ticked`, `ticked2`, `searcherdummy`.

---

## Layer / SplineSet / Spline / SplinePoint / RefChar / ImageList

### Layer — `splinechar.h:414` (`struct reflayer`, `:216`, is its prefix-compatible subset)
- `background`: layer contributes no width and is not exported.
- `order2`: this layer's splines are quadratic (TrueType) rather than cubic.
- `dofill, dostroke, fillfirst, fill_brush` (`struct brush`: color, opacity, pattern, gradient), `stroke_pen` (`struct pen`: brush, linejoin, linecap, width, `trans[4]`, `dashes[8]`) — Type3/SVG paint.
- `splines` (`SplinePointList*`), `refs` (`RefChar*`, foreground layers only), `images` (`ImageList*`, background/Type3 only), `guidelines` (`GuidelineSet*`, UFO-only).
- `undoes` / `redoes`: **undo stacks are per-layer**, not per-glyph.
- `validation_state` / `old_vs` (`enum validation_state`, `splinefont.h:898`), `python_persistent`.
- `anyflexes`: some spline in this layer uses PostScript flex.

### LayerInfo — `splinefont.h:1089`
`name`, `background`, `order2` (applies to *all glyphs* in the font for that layer index), `ufo_path`, `ticked`. `BACK_LAYER_MAX = 256` caps normal fonts. `enum layer_type` (`splinefont_enums.h:143`): `ly_all=-2, ly_grid=-1, ly_back=0, ly_fore=1, ly_none=-3`.

### SplinePointList / SplineSet — `splinechar.h:171`
`first`, `last` (`last == first` and `first->prev != NULL` ⇒ closed contour); `next` (contours chain into a list); `spiros` / `spiro_cnt` / `spiro_max` (parallel spiro control-point array — the last entry is a terminator, so real count is `spiro_cnt-1`); `beziers_need_optimizer`; `is_clip_path` (Type3/SVG); `start_offset` (which point is canonical first for UFO output); `contour_name`.

### SplinePoint — `splinechar.h:134`, `enum pointtype` at `:133`
- `me`, `nextcp`, `prevcp`: on-curve point and the two control points (absolute coordinates, not deltas).
- `nonextcp` / `noprevcp`: control point coincides with `me` (that side is a straight segment).
- `nextcpdef` / `prevcpdef`: control point is at its automatically-computed default.
- `pointtype` (2 bits): `pt_curve` (smooth, cps colinear and equidistant-ish), `pt_corner`, `pt_tangent` (one side straight, other tangent to it), `pt_hvcurve` (curve constrained to horizontal/vertical tangents).
- `ttfindex`: TrueType point number. Sentinels: `0xffff` = point is implied by averaging two adjacent quadratic control points (not stored in `glyf`); `0xfffe` = not yet numbered.
- `nextcpindex`: TrueType point number of `nextcp`.
- `roundx` / `roundy`, `dontinterpolate`: TrueType hinting hints.
- `hintmask` (`HintMask*` = 96-bit bitset, `splinechar.h:130`): PostScript hint-replacement mask that becomes active *at* this point.
- `name`: UFO point name; `isintersection`, `flexx`/`flexy`; `next` / `prev` (`Spline*`).
- Selection bits (`selected`, `nextcpselected`, `prevcpselected`) are UI-only, but note quadratic control-point selection uses the convention "`nextcpselected` on the from-point" (`splineorder2.c:1300`).

### Spline1D / Spline — `splinechar.h:453` / `:465`
`Spline1D {real a,b,c,d}`. `Spline.splines[0]` is x, `[1]` is y; the curve is
`p(t) = a·t³ + b·t² + c·t + d`, `t ∈ [0,1]`.

Cubic fill (`splinerefigure.c:46`, `SplineRefigure3`):
`d = from.me`, `c = 3(from.nextcp − from.me)`, `b = 3(to.prevcp − from.nextcp) − c`, `a = to.me − from.me − c − b`.

Quadratic fill (`splineorder2.c:1234`, `SplineRefigure2`): `d = from.me`, `c = 2(from.nextcp − from.me)`, `b = to.me − from.me − c`, **`a = 0`**. Quadratics additionally require `from.nextcp == to.prevcp` — that shared point *is* the single off-curve control point, duplicated into both endpoints so all geometry code can treat every spline as a cubic.

Flags: `order2` (authoritative: which refigure routine applies), `isquadratic` (a *cubic* whose `a` happens to be 0), `islinear`/`knownlinear`/`knowncurved`, `ishorvert`, `acceptableextrema` (user has approved extrema; invalidated if b/c coefficients change), plus remove-overlap scratch bits (`isneeded`, `isunneeded`, `exclude`, `touched`, `leftedge`, `rightedge`, `isticked`). `approx` (`LinearApprox*`, `:444`) is a per-zoom-scale cached polyline — pure cache.

`SplineRefigure()` (`splineorder2.c:1329`) dispatches on `spline->order2`. Conversion is *approximation*, not exact: `SSttfApprox`/`SplineSetsTTFApprox` (cubic→quadratic, `splineorder2.c:858/897`) and `SSPSApprox`/`SplineSetsPSApprox` (quadratic→cubic, `:960/1007`), driven by `SplineSetsConvertOrder`, `SCConvertLayerToOrder2/3`, `SFConvertLayerToOrder2/3`, `SFConvertGridToOrder2/3` (`splineorder2.h`). Quadratic→cubic is exact per segment; cubic→quadratic subdivides until `comparespline` error is under threshold.

### RefChar — `splinechar.h:229`
- `sc`: resolved target glyph; `orig_pos`: target GID (the on-disk form); `encoded` says `orig_pos` is actually an old-SFD encoding value; `unicode_enc`, `adobe_enc` (Adobe StandardEncoding slot, used for Type1 `seac`).
- `transform[6]`: 2×3 affine (rows of a 3×3 with implied `0,0,1`). `justtranslated` caches "matrix is a pure translation/identity".
- `use_my_metrics`: TrueType composite flag — take the advance width from this component.
- `round_translation_to_grid`: TrueType `ROUND_XY_TO_GRID`.
- `point_match` + `match_pt_base`, `match_pt_ref`: TrueType point-matching attachment instead of an offset; `point_match_out_of_date` when a base glyph was edited.
- `layers` / `layer_cnt` (`struct reflayer`): **cached flattened, already-transformed copies** of the target's outlines. For a normal font this is exactly 1 layer holding the target's splines *plus recursively all of the target's own references* flattened in; for multilayer Type3 fonts one reflayer per contributing source layer (`splineutil.c:3082-3181`). `bb`, `top` are cached bounds.
- `SCReinstanciateRefChar` rebuilds these caches; `_SFReinstanciateRefs` (`splineutil.c:3184`) does a tick-based topological sweep, iterating up to 200 rounds so nested references resolve bottom-up.

### ImageList — `splinechar.h:188`
`image` (`struct gimage*`, `inc/gimage.h`), `xoff`/`yoff` (upper-left in glyph space), `xscale`/`yscale` (glyph units per image pixel), `bb`, `next`, `selected`.

### GuidelineSet — `splinechar.h:200`
UFO-only per-glyph guideline: `name`, `identifier`, `point`, `angle`, `color` (RGBA), `flags`. The header explicitly says FontForge's own mechanism is the font-global `sf->grid`.

---

## Bitmaps

### BDFFont — `splinefont.h:812`
`sf` (owning outline font), `glyphs`/`glyphcnt`/`glyphmax`, `pixelsize`, `ascent`/`descent`, `clut` (`GClut*`, for antialiased strikes), `foundry`, `res`, `props`/`prop_cnt` (`BDFProperties{name,type,union{str,atom,val}}`, `:802`, `enum property_type` `:800`), `truesize` + `bbsized` (strike sized by bounding box rather than em), `piecemeal` + `layer` + `ptsize` + `dpi` + `freetype_context` (strike rendered lazily via FreeType), `unhinted_freetype`, `recontext_freetype`, `next`.

**Relationship to the outline font**: `bdf->glyphs` is indexed by the *same GID* as `sf->glyphs` (see `bitmapcontrol.c:201`), so strikes are parallel arrays, not independently encoded. Strikes hang off `sf->bitmaps` in a list kept sorted by `pixelsize` (`SFOrderBitmapList`). For CID fonts the bitmaps live on the `cidmaster`, not the subfonts (`encoding.c:1462`).

### BDFChar — `splinechar.h:283`
`sc` (back-pointer), `orig_pos`, `xmin/xmax/ymin/ymax` (ink box), `width`, `vwidth`, `bytes_per_line`, `bitmap` (MSB-first bits, or one byte per pixel when `byte_data`), `depth`, `refs` (`BDFRefChar*`, `:260`: `{xoff,yoff,gid,bdfc}` — bitmap composites), `dependents`, `selection`/`backup` (`BDFFloat*`), `undoes`/`redoes`, `widthgroup`/`isreference` (TTF EBDT output hints), `pixelsize` (for undo).

### BDFFloat — `splinechar.h:275`
A floating (selected/pasted, not yet merged) sub-bitmap: `xmin/xmax/ymin/ymax`, `bytes_per_line`, `byte_data`, `depth`, `bitmap`.

---

## Kerning, anchors, lookups, substitutions

- **KernPair** (`splinefont.h:468`): `subtable`, `sc` (the *right* glyph), `off` (int16), `adjust` (`DeviceTable*` for one dimension only), `kcid` (temp), `next`. Owned by the left glyph.
- **KernClass** (`splinefont.h:486`): `first_cnt`/`second_cnt`, `firsts[]`/`seconds[]` (space-separated glyph-name lists; entry 0 is NULL = "everything else"), `firsts_names`/`seconds_names` + `firsts_flags`/`seconds_flags` (UFO storage/name-format flags, `FF_KERNCLASS_FLAG_NATIVE|FEATURE|NAMETYPE|NAMELEGACY|VIRTUAL|FLATTEN`), `offsets[first_cnt*second_cnt]` row-major int16 (0 = no data), `offsets_flags`, `adjusts[]` (parallel DeviceTables), `subtable`, `feature`, `next`.
- **AnchorClass** (`splinefont.h:444`), `enum anchorclass_type` (`:443`): `act_mark, act_mkmk, act_curs, act_mklg`. Fields: `name`, `subtable`, `type`, `has_base`, `has_mark`, `ac_num`.
- **AnchorPoint** (`splinefont.h:455`), `enum anchor_type` (`:454`): `at_mark, at_basechar, at_baselig, at_basemark, at_centry, at_cexit`. Fields: `anchor` (class), `me`, `xadjust`/`yadjust` (DeviceTables), `type`, `lig_index` (which ligature component, for `at_baselig`), `ttf_pt_index` + `has_ttf_pt` (TrueType point-attached anchors).
- **OTLookup** (`splinefont.h:398`): `lookup_type` (`enum otlookup_type`, `:277`), `lookup_flags` (low 16 = `enum pst_flags` + mark class in `0xff00`; high 16 = mark-filtering-set index), `lookup_name`, `features` (`FeatureScriptLangList*`), `subtables`, `store_in_afm`, `in_gpos`, `in_jstf`, `only_jstf`, plus generation-time `lookup_index/offset/length/subcnt`.
- **lookup_subtable** (`splinefont.h:366`): `subtable_name`, `suffix` (default-replacement suffix for `gsub_single`), `separation`/`minkern`/`kerning_by_touch`/`onlyCloser`/`dontautokern` (autokern params for `gpos_pair`), `lookup` (parent), exactly one of `kc` / `fpst` / `sm` for class-kerning / contextual / state-machine subtables, and the classification bits `per_glyph_pst_or_kern`, `anchor_classes`, `vertical_kerning`. A kern class, FPST or ASM gets a subtable to itself; anchor classes may share; per-glyph PSTs may share. `extra_subtables` holds offsets when an oversized kern subtable is split (terminated by −1).
- **FeatureScriptLangList** (`splinefont.h:352`) + **scriptlanglist** (`:334`): `featuretag` → list of `{script, langs[4] + morelangs, lang_cnt}`. `ismac` reinterprets `featuretag` as a packed Mac feature/setting. Defaults: `DEFAULT_LANG='dflt'`, `DEFAULT_SCRIPT='DFLT'`, `REQUIRED_FEATURE=' RQD'`.
- **PST / generic_pst** (`splinefont.h:504`), `enum possub_type` (`splinechar.h:332`): tagged union keyed by `type` —
  - `pst_position` → `u.pos` (`struct vr {xoff,yoff,h_adv_off,v_adv_off, ValDevTab *adjust}`, `shapers/opentype_str.h:34`)
  - `pst_pair` → `u.pair {char *paired; struct vr *vr;}` (two value records)
  - `pst_substitution` → `u.subs.variant` (one glyph name)
  - `pst_alternate` / `pst_multiple` → `u.alt/mult.components` (space-separated names)
  - `pst_ligature` → `u.lig {components, lig}`; stored **on the ligature glyph**, listing its components
  - `pst_lcaret` → `u.lcaret {int16_t *carets; int cnt}` (GDEF ligature caret positions)
  - Values past `pst_max` are not PSTs but reuse the enum space: `pst_kerning, pst_vkerning, pst_anchors`, the five FPST types (`pst_contextpos, pst_contextsub, pst_chainpos, pst_chainsub, pst_reversesub`), and the copy/paste kludges `pst_kernback, pst_vkernback`.
- **FPST / generic_fpst** (`splinefont.h:554`): `type` (one of the five contextual `possub_type`s), `format` (`enum fpossub_format`, `:528`: `pst_glyphs, pst_class, pst_coverage, pst_reversecoverage`), class lists `nclass/bclass/fclass` (+ `*classnames`) with counts, and `rules[rule_cnt]` (`struct fpst_rule`, `:541`) whose union is `fpg{names,back,fore}` / `fpc{n,b,f class index arrays}` / `fpv{n,b,f coverage lists}` / `fpr{... , replacements}`. Each rule carries `lookups[lookup_cnt]` of `seqlookup{seq, lookup}`. **Backtrack (`b*`) arrays are stored in OpenType order, i.e. reversed** relative to display order.
- **ASM / generic_asm** (`splinefont.h:591`), `enum asm_type` (`:568`): `asm_indic, asm_context, asm_lig, asm_simple=4, asm_insert, asm_kern=0x11`. Fields: `flags` (`asm_vert=0x8000, asm_descending=0x4000, asm_always=0x2000`), `class_cnt`, `state_cnt`, `classes[]` (glyph-name lists), `state[class_cnt*state_cnt]` of `asm_state{next_state, flags, union{context{mark_lookup,cur_lookup}, insert{mark_ins,cur_ins}, kern{kerns,kcnt}}}`. The per-type meaning of `flags` bits (rearrangement verbs 0–f, insert counts, kern stack) is documented in the comment at `:602-632`.
- **DeviceTable** (`:431`): `{first_pixel_size, last_pixel_size, int8_t *corrections}`; **ValDevTab** (`:436`) bundles four of them (xadjust, yadjust, xadv, yadv).

---

## EncMap / Encoding — `splinefont.h:786` / `:737`

- `Encoding`: a *shared, named* codepage definition — `enc_name`, `char_cnt`, `unicode[]` (codepoint per slot), `psnames[]` (optional PS name per slot), iconv handles `tounicode`/`fromunicode` or function pointers, ISO-2022 escape, and a pile of classification bits (`is_unicodebmp`, `is_unicodefull`, `is_custom`, `is_original`, `is_compact`, `is_japanese/korean/tradchinese/simplechinese`, `only_1byte`, `builtin`, `hidden`, `is_temporary`).
- `EncMap`: the *per-font instance* — `map[enc] → gid` (`enccount`/`encmax`), `backmap[gid] → enc` (`backmax`), `enc` (pointer to the shared Encoding), and `remap` (`struct remap{firstenc,lastenc,infont}`, `:784`) for compacted views. `-1` means unmapped. `EncMap` is what the font view iterates; the glyph array itself is index-by-GID and encoding-agnostic.
- CID fonts have **no encoding**: the slot index *is* the CID, and names/unicode come from an external CIDMap file (`SCBuildDummy`, `splinefont.c:106-127`).

---

## MMSet — `splinefont.h:1373`

`axis_count` (≤4), `axes[4]` (axis tag strings), `instance_count`, `instances[]` (each a full `SplineFont`), `normal` (the default instance — also a `SplineFont`), `positions[instance][axis]` (design-space location), `defweights[instance]` (Adobe blend weights making up `normal`), `axismaps[axis]` (`struct axismap`, `:1354`: `points`, `blends[]` in [0,1] ascending, `designs[]` in design units, `min/def/max`, `axisnames` for Mac), `cdv`/`ndv` (Adobe ConvertDesignVector / NormalizeDesignVector PostScript source), `named_instance_count`/`named_instances` (`:1362`: `coords[axis]` + Mac `names`), `apple` (Apple `var` tables vs Adobe MM), `changed`. Limits: `MmMax=16`, `AppleMmMax=26` (`splinechar.h:506`). Intermediate masters are deliberately unsupported.

On import, OpenType variations parse into `struct variations` (`ttf.h:69`): `taxis{tag,min,def,max,nameid,paircount,mapfrom[],mapto[]}` (fvar+avar), `tinstance{nameid, coords[]}`, `tuples{coords[], chars[], cvt, kern classes}` (gvar/cvar deltas materialized as whole varied glyph sets).

---

## Hinting structures

- **StemInfo** (`splinechar.h:513`): `start` (position along the minor axis), `width` (or height; `ghost` stems store 20 or 21 and mean an edge hint), `where` (`HintInstance*` list, `:496`: `{begin, end, closed, counternumber}` — the ranges along the *other* axis where this stem is active, because e.g. the stems of "H" vanish at the crossbar), `hintnumber` (bit index in `HintMask`), `hasconflicts`, `haspointleft/right`, `u.mask` (references using this hint in Type2 output) or `u.unblended` (`_MMArray = real[2][16]`, MM hint values), plus autohint/instruct scratch bits.
- **DStemInfo** (`splinechar.h:559`): first two fields are layout-compatible with `StemInfo`; then `left`, `right`, `unit` (BasePoints defining the diagonal), and `where`.
- **MinimumDistance** (`splinechar.h:568`): `sp1`, `sp2` (NULL means origin or width point), `x` (horizontal vs vertical constraint), `done`.
- **HintMask** = `uint8_t[12]` (96 bits, `splinechar.h:131`), attached to `SplinePoint.hintmask` and to `SplineChar.countermasks[]`.
- **BlueData** (`splinefont.h:251`) and **GlobalInstrCt**/**BlueZone**/**StdStem** (`:1593-1631`) are the derived alignment-zone/stem-snap structures used by autohinting and auto-instructing.

---

## Undo model — `Undoes`, `splinechar.h:361`; logic in `cvundoes.c`

`enum undotype` (`splinechar.h:308`): `ut_none, ut_state, ut_tstate, ut_statehint, ut_statename, ut_statelookup, ut_anchors, ut_width, ut_vwidth, ut_lbearing, ut_rbearing, ut_possub, ut_hints, ut_bitmap, ut_bitmapsel, ut_composit, ut_multiple, ut_layers, ut_noop`.

Common header: `next`, `undotype`, `was_modified` (the glyph's dirty flag before the edit), `was_order2` (so undoing across an order conversion restores the right spline kind), `layer` (`UNDO_LAYER_UNKNOWN = -1`), `copied_from` (source `SplineFont*`, needed for cross-font paste).

Union payloads:
- `u.state` — full snapshot: `width`, `vwidth`, `lbearingchange`, `unicodeenc`, `charname`, `comment`, `possub` (the last four only for `ut_statename`), `splines`, `refs`, `images`, `hints` (opaque blob from `UHintCopy`), `instrs`+`instrs_len`, `anchor`, `fill_brush`, `stroke_pen`, `dofill`/`dostroke`/`fillfirst`.
- `u.width` / `u.lbearing` / `u.rbearing` — scalars.
- `u.bmpstate` — an embedded `BDFChar` by value (bitmap undo).
- `u.composit {state, bitmaps}` — an outline undo plus a chain of bitmap undoes (used for clipboard copies of whole glyphs).
- `u.multiple.mult` — a chain of sub-undoes; also used by `ut_layers`, where sub-undo *i* corresponds to layer `ly_fore + i`.
- `u.possub {pst, data[], more_pst, cnt, max}` — `data` entries are "4-byte tag, space, payload".

Mechanics: `AddUndo` (`cvundoes.c:581`) pushes onto `sc->layers[layer].undoes` and clears the redo stack; `SCUndoAct` (`:914`) is a **swap**, not a restore — it exchanges live state with the undo record so the same record becomes the redo record. `SCPreserveState` (`:746`) snapshots `ly_fore`, and for multilayer fonts also every layer above it. `_SFPreserveGuide` (`:761`) pushes onto `sf->grid.undoes`, so guides have their own undo stack. Font-level undo is separate: `SFUndoes` (`splinefont.h:726`) is a doubly-linked (`dlistnode`) stack of `{msg, type ∈ sfut_lookups|sfut_lookups_kerns|sfut_fontinfo|sfut_noop, sfdchunk}` — the payload is a **serialized SFD fragment**, not a struct graph.

---

## Enums worth porting (`splinefont_enums.h` and neighbours)

- `enum fontformat` (`:34`): pfa, pfb, pfbmacbin, multiple, mma, mmb, ptype3, ptype0, cid, cff, cffcid, type42, type42cid, ttf, ttfsym, ttfmacbin, ttc, ttfdfont, otf, otfdfont, otfcid, otfciddfont, svg, ufo, ufo2, ufo3, woff_ttf, woff_otf, woff2_ttf, woff2_otf, none — plus predicates `isttf_ff`, `isttflike_ff`. Order is load-bearing (tables are indexed by it).
- `enum bitmapformat` (`:72`): bdf, ttf, sfnt_dfont, sfnt_ms, otb, nfntmacbin, fon, fnt, palm, ptype3, none.
- `enum ttf_flags` (`:86`) and `enum ps_flags` (`:123`) share one 32-bit space and must not collide.
- `enum openflags` (`:115`), `enum ttc_flags` (`:113`), `enum layer_type` (`:143`).
- `splinechar.h`: `enum pointtype` (`:133`), `enum undotype` (`:308`), `enum possub_type` (`:332`), `enum hinttypes` (`:504`, `ht_unspecified/h/v/d`), `enum spreadMethod` (`:80`), `enum image_type`.
- `splinefont.h`: `enum linejoin` (`:105`: miter, round, bevel, miterclip, nib, arcs, inherited), `enum linecap` (`:114`: butt, round, square, nib, bevel, inherited), `enum stroke_rmov` (`:122`), `enum stroke_arclimit` (`:127`), `enum si_type` (`:136`: round, calligraphic, nib, centerline), `enum overlap_type` (`:163`), `enum simpify_flags` (`:166`), `enum otlookup_type` (`:277`) and `enum otlookup_typemasks` (`:310`), `enum pst_flags` (`:361`), `enum anchorclass_type`/`anchor_type` (`:443`/`:454`), `enum fpossub_format` (`:528`), `enum asm_type`/`asm_flags` (`:568`/`:570`), `enum otffn_field` (`:673`), `enum sfundotype` (`:705`), `enum uni_interp` (`:781`), `enum property_type` (`:800`), `enum privatedict_state` (`:876`), `enum validation_state` (`:898`), `enum ttfnames` (`:949`), `enum backedup_state`/`loadvalidation_state` (`:1075`/`:1076`), `enum texdata_type` (`:1169`), `enum style_flags` (`:1391`), `enum pconvert_flags` (`:1504`), `enum transformPointType`/`transformPointMask` (`:1534`/`:1539`).
- `baseviews.h`: `enum drawmode {dm_grid, dm_back, dm_fore, dm_max}` (`:134`) — the three "layer heads" a glyph editor tracks; `enum widthtype` (`:41`), `enum fvcopy_type` (`:299`), `enum fvformats` (`:283`).

`enum otlookup_type` encoding is the key one: `value & 0xff` is the OpenType lookup type and `value >> 8` selects the table (0 = GSUB, 1 = GPOS). Apple state machines are smuggled in as `morx_indic/context/insert = 0x0fd..0x0ff` (GSUB side) and `kern_statemachine = 0x1ff` (GPOS side).

---

## Import-time structures (not the persistent model)

- `struct ttfinfo` (`ttf.h:92`): a ~200-field scratch record holding every table's file offset/length plus partially-built model objects (`chars`, `layers`, `bitmaps`, `ahead/alast`, `khead/klast/vkhead/vklast`, `gsub_lookups/gpos_lookups`, `possub`, `sm`, `features`, `variations`, `math`, `guidelines`, `horiz_base/vert_base`, `justify`) and a set of `bad_*` bits that become `SplineFont.loadvalidation_state`.
- `psfont.h`: `FontDict` (`:105`) mirrors a Type1/CFF font dictionary (`fontmatrix`, `encoding[256]`, `fontbb`, `painttype`, `chars`/`charprocs`, `psprivate`, CID `fds[]`/`cidstrs`/`cidfds`, MM `blendfunc`/`weightvector`/`cdv`/`ndv`); `struct cidbytes`/`fddata`/`fd2data` are the CID/Type2 emit-side helpers.
- `sd.h`: the intermediate "entity" tree used when importing arbitrary PostScript/SVG art before it becomes splines — `Entity` (`:95`) is a tagged union over `et_splines` (`filledsplines`: splines + fill/stroke `Pen`, width, miterlimit, join, cap, transform), `et_text`, `et_image`, `et_group`, each with `clippath` and `bb`; `EntLayer` (`:108`) groups them.
- `PfEd.h`: the private `PfEd` sfnt table format that lets FontForge round-trip editor state through a compiled TTF/OTF — subtables `fcmt` (font comment), `flog` (FONTLOG), `cmnt` (per-glyph comments, v1 = utf-8), `cvtc` (cvt comments), `colr` (glyph colors as glyph ranges), `GPOS`/`GSUB` (lookup/subtable/anchor *names*), `guid` (guidelines), `layr` (background/spiro layers). Contours are encoded with a compact verb/modifier bytecode (`V_MoveTo|V_B` etc., verbs at `V_MoveTo=0` … `V_End=45`, modifiers `V_B/V_S/V_F` selecting byte/short/16.16-fixed operands), including quadratic implicit-point verbs `V_QImplicit`/`V_QHImplicit`/`V_QVImplicit`.

---

# Design notes for a Rust port

1. **`order2` is a per-layer property, not per-font and not per-spline-in-isolation.** `LayerInfo.order2` (`splinefont.h:1092`) is the source of truth for "all glyphs in the font, this layer index"; `Layer.order2` and `Spline.order2` are copies kept in sync. A font can legitimately have a quadratic foreground and a cubic background simultaneously. Model it as `enum SplineOrder { Cubic, Quadratic }` on the layer and derive it on splines, and keep `Undoes.was_order2` so undo across a conversion is sound.

2. **Both representations share one `Spline1D` cubic form.** Quadratics are stored as cubics with `a == 0` (`splineorder2.c:1290`) *and* with the single control point duplicated into `from.nextcp` and `to.prevcp`. Every geometry routine (bounds, intersection, solving, stroking) then works on cubics unconditionally. A Rust port gets the same win by storing `CubicBez` internally and only splitting on export; but you must preserve the "the two control points are literally equal" invariant, because TrueType point numbering depends on it.

3. **Coefficients are *cached derivatives* of the point data, not the primary representation.** The comment at `splinechar.h:457-463` warns that mutating `from.me` requires re-running `SplineRefigure`. In Rust, make coefficients a memoized value computed from the four points rather than a mutable sibling field, and make `SplineRefigure` unnecessary by construction.

4. **The `0xffff` / `0xfffe` `ttfindex` sentinels encode "this on-curve point does not exist in the font file".** TrueType lets consecutive off-curve points imply an on-curve midpoint; FontForge materializes those implied points in memory and tags them so they are dropped on write. Any port that keeps a faithful `glyf` round-trip needs an explicit `enum TtfPointIndex { Implied, Unnumbered, Index(u16) }`.

5. **References cache a fully flattened, transformed copy of their target.** `RefChar.layers[]` is not the reference definition — it is a derived render cache that `SCReinstanciateRefChar` regenerates, recursively inlining the target's own references (`splineutil.c:3166-3179`). `_SFReinstanciateRefs` resolves the dependency DAG by repeated ticked sweeps capped at 200 iterations, i.e. FontForge tolerates (and silently gives up on) reference cycles. A Rust port should keep references as pure `{target, transform, flags}` and compute flattening on demand, and should detect cycles properly.

6. **The reverse dependency index is materialized.** `SplineChar.dependents` lets an edit to a base glyph invalidate composites in O(dependents) rather than O(font). Combined with `RefChar.point_match_out_of_date`, this is how point-matched composites stay correct. In Rust this is the classic "store the graph, derive the reverse index" problem — consider an arena/index-based graph (`GlyphId` keys) rather than pointers.

7. **CID fonts are modeled as a `SplineFont` whose `glyphs` array is empty and whose `subfonts[]` hold the real glyphs.** Lookup walks subfonts for the first one with `gid < glyphcnt` and a non-NULL entry (`splinefont.c:176-186`). The CID *is* the array index — there is no encoding indirection. Almost all font-level data (lookups, kerning, names, OS/2, TeX, MATH, bitmaps, FPSTs, Mac features) lives on the `cidmaster`, while only glyph data and the Private dict live in subfonts (`encoding.c:1462-1487`). Consider modeling this as `enum GlyphSource { Flat(Vec<Glyph>), Cid(Vec<Subfont>) }` on one font struct rather than a self-referential parent/child pair.

8. **`layers` indexes into *two* parallel arrays.** `sf->layers[i]` (`LayerInfo`: name, background, order2) is the *definition*; `sc->layers[i]` (`Layer`: splines, refs, images, undo) is the *content*. They are kept parallel by `SFAddLayer`/`SFRemoveLayer` (`splinefont.c:1904-1990`), which must edit every glyph. Crucially, `sc->layer_cnt` may be **less** than `sf->layer_cnt` (the removal code explicitly guards `if (l < sc->layer_cnt)`), so a port must treat missing trailing layers as empty rather than assuming equal lengths.

9. **The grid/guide layer is a `Layer` embedded by value on `SplineFont`, addressed by the out-of-band index `ly_grid == -1`.** Code paths written for glyph layers special-case it: `SCUndoAct` does `layer==ly_grid ? &sc->parent->grid : &sc->layers[layer]` (`cvundoes.c:947`). It has its own undo stack and its own `order2` flag, and `SFConvertGridToOrder2/3` converts it separately. A Rust port is better served by a distinct `Guides` type than by a negative index into a layer array.

10. **Per-layer undo stacks with swap semantics.** Undo records live on `Layer.undoes`/`Layer.redoes`, and `SCUndoAct` swaps live state into the record rather than copying out of it, so one allocation serves as both undo and redo. This is elegant in C but hostile to Rust's ownership model; prefer immutable snapshots (or a persistent data structure / command log) and accept the extra allocation.

11. **Kerning is asymmetric by ownership.** `KernPair` hangs off the *left* glyph and names the right one (`splinefont.h:469`). Class kerning is a completely separate representation (`KernClass` with a dense `first_cnt × second_cnt` int16 matrix, class 0 = "everything else"), and pair kerning is synthesized from classes only for legacy output (`OTLookup.temporary_kern`). Don't unify them prematurely; the UFO round-trip flags on `KernClass` (`FF_KERNCLASS_FLAG_*`) exist precisely because the two worlds don't map cleanly.

12. **Lookup → subtable → item is a three-level hierarchy with placement rules encoded in the subtable, not the item.** The comment at `splinefont.h:382-388` spells out the invariants: a kern class / FPST / state machine owns its subtable exclusively; anchor classes may share; per-glyph PSTs may share; and an item can only go into a lookup whose type it fits. A Rust port should make this a type-level guarantee (`enum SubtableContent { PerGlyph(Vec<...>), KernClass(..), Fpst(..), Asm(..), Anchors(..) }`) instead of four nullable pointers plus three booleans.

13. **`enum possub_type` is deliberately over-loaded past `pst_max`** to double as an identifier for kerning, anchors, FPST types and cut/paste kludges (`splinechar.h:344-358`). Splitting these into separate Rust enums is correct, but note that SFD files and the clipboard serialize the numeric values, so a compatibility layer needs the exact mapping.

14. **Unparsed table blobs are first-class.** `struct ttf_table` (`splinefont.h:1161`) keeps `fpgm/prep/cvt/maxp` in `ttf_tables` and arbitrary user-preserved tables in `ttf_tab_saved`, and `SplineFont.python_persistent` / `Layer.python_persistent` keep opaque pickled payloads that are written back unchanged. Any serious editor needs this "carry unknown data through unmodified" escape hatch; design for it from the start rather than bolting it on.

15. **A large fraction of every struct is derived state or scratch space, and it is not marked.** `ticked`/`ticked2` (graph traversal marks), `kcid`, `lsidebearing`, `ttf_glyph`, `hintnumber`, `subtable_offset`, `lookup_index`, `nid`, `strid`, `Spline.approx`, `RefChar.bb/top/layers`, `LigList`, `SplineChar.glyphnames` hash, `BDFFont.freetype_context` are all recomputable or transient. Separating "document model" from "derived index" from "render cache" is the single largest structural improvement available to a port — FontForge's `ticked` flags in particular exist only because the C code has no way to carry a visited-set alongside a traversal.
