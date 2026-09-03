<!-- Research report generated 2026-09-03 by an Explore agent reading fontforge/ (FontForge git dc05099f, 2026-08-10). Paths are relative to fontforge/. -->

# FontForge Font-Format I/O — Complete Inventory

Source root: `/home/chen/githubprojects/pixelfonts/fontworkshop/fontforge/`

The three enums that drive everything live in `fontforge/fontforge/splinefont_enums.h`:
`enum fontformat` (line 34, 30 members `ff_pfa` … `ff_none`), `enum bitmapformat` (line 71, 11 members `bf_bdf` … `bf_none`), `enum ttf_flags` (line 87), `enum ps_flags` (line 123), `enum ttc_flags` (line 113), `enum openflags` (line 115). Save-side dialog flags are a *separate* bitfield, `enum fm_flags` in `fontforge/fontforge/savefont.h:38-67`, translated by `fmflag2ttfflag()` (`savefont.c:1039-1076`).

Two parallel extension tables index directly by those enums — `savefont_extensions[]` and `bitmapextensions[]`, `fontforge/fontforge/savefont.c:65-97`. The comment at line 78 is load-bearing: *"the order here must match the fontformat enum."*

---

## 1. Master inventory

| Format | Ext(s) | Read | Write | Source files | Options / notes |
|---|---|---|---|---|---|
| **SFD (Spline Font Database)** | `.sfd`, `.sfd~` | ✔ | ✔ | `sfd.cpp` (9534 lines), `sfd1.c` (old→new conversion) | Native. Version written is 3.0/3.1/3.2 (`sfd.cpp:2956-2965`) — 3.2 iff `WritePNGInSFD`, 3.1 if undo-limit saving, else 3.0. Parser accepts 0,1,2,3,3.1–3.21,4 (`sfd.cpp:9026-9031`). Optional gzip/bz2/xz/Z compression on save via `sf->compression` (`sfd.cpp:3158-3203`). |
| **SFDir** (exploded SFD) | `.sfdir` dir | ✔ | ✔ | `sfd.cpp` (`todir` arg), `SFDirRead` | Directory with `font.props`, `*.glyph`, `<size>.strike/strike.props`, `*.bitmap`, `*.subfont`, `mm<N>.instance` (`sfd.cpp:135-142`). Detected by presence of `font.props` (`splinefont.c:1072-1077`). |
| **PS Type 1 ASCII** | `.pfa` | ✔ | ✔ | `parsepfa.c`, `psread.c`, `dumppfa.c` | `ff_pfa`. |
| **PS Type 1 binary** | `.pfb` | ✔ | ✔ | same | `ff_pfb`; PFB section headers `0x80 0x01` sniffed at `splinefont.c:1110-1113`. |
| **PS Type 1 Mac resource** | `.bin` (`.res` on Mac) | ✔ | ✔ | `macbinary.c:1569 WriteMacPSFont`, `SearchPostScriptResources:1950` | `ff_pfbmacbin`, `POST` resource. Requires ≥1 NFNT bitmap to accompany it (dialog error, `savefontdlg.c:1442`). Warns 'POST' is deprecated (`:1446`). |
| **PS Type 1 "Multiple"** | `%s.pfb` / `.mult` | — | ✔ | `savefont.c:721 WriteMultiplePSFont`, `ParseWernerSFDFile:562` | Splits a CJK font into ≤256-glyph subfonts driven by a **TeX SubFont Definition** file (also `.sfd`! guarded against confusion at `savefont.c:441-445`). |
| **Multiple Master (A/B)** | `.mm.pfa` / `.mm.pfb` | ✔ | ✔ | `mm.c`, `multimastersubrs.c`, `dumppfa.c` | `ff_mma`/`ff_mmb`. Also writes `.amfm` plus one `.afm` per instance (`savefont.c:129-168`). |
| **PostScript Type 3** | `.pt3`, `.pf3` | ✔ | ✔ | `psread.c`, `dumppfa.c:1869+` | Multilayer/stroked. `CheckIfTransparent()` (`savefont.c:766`) refuses translucent layers — Type3 has no alpha. |
| **PostScript Type 0** | `.ps` | ✔ | ✔ | `dumppfa.c:2026 dumptype0stuff` | `ff_ptype0`, composite font for 2-byte encodings. |
| **PS CID-keyed Type 1** | `.cid` | ✔ | ✔ | `dumppfa.c:dumpcidstuff`, `parsepfa.c` | `ff_cid`. |
| **Bare CFF** | `.cff`, `.cef`, `.gai` | ✔ | ✔ | `parsettf.c:_CFFParse/CFFParse`, `tottf.c:dumpcff` | Magic `01 00 04` (`splinefont.c:1117-1123`). `ps_flag_nocffsugar` strips the PS wrapper when embedding into OTF/PDF. |
| **Bare CFF CID** | `.cid.cff` | ✔ | ✔ | same | `ff_cffcid`. |
| **Type 42** | `.t42` | ✔ | ✔ | `dumppfa.c:2080-2170`, `tottf.c` (sfnt embedded) | `ff_type42`. TrueType outlines wrapped in PostScript. |
| **Type 11 / CID Type 2** | `.t11`, `.cid.t42` | ✔ | ✔ | `dumppfa.c:2083-2100` | `ff_type42cid`; `ps_flag_identitycidmap` forces gid==cid. |
| **TrueType** | `.ttf` | ✔ | ✔ | `parsettf.c`, `parsettfatt.c`, `parsettfbmf.c`, `parsettfvar.c`, `tottf.c`, `tottfgpos.c`, `tottfaat.c`, `tottfvar.c`, `ttfspecial.c` | Sniffed by `00 01 00 00` / `true` (`splinefont.c:1096-1101`). Full flag matrix below. |
| **TrueType (Symbol)** | `.ttf`, `.sym.ttf` | ✔ (auto) | ✔ | `tottf.c:3307,3994,4875-4934`; read side `parsettf.c:5699 SymbolFixup` | `ff_ttfsym` / `ttf_flag_symbol`. Emits a (3,0) cmap in the `0xF000-0xF0FF` PUA range; `OS/2.firstcharindex/lastcharindex` are biased by `0xF000` (`tottf.c:3469`). On read, `info->twobytesymbol` triggers `SymbolFixup` remapping `0xF0xx`→`0x00xx`. |
| **TrueType MacBinary / suit** | `.ttf.bin`, `.suit` | ✔ | ✔ | `macbinary.c:1651 WriteMacTTFFont`, `:2073 SearchTtfResources` | `sfnt`+`NFNT`+`FOND` resources in a MacBinary-3 wrapper (`DumpMacBinaryHeader:1374`). |
| **Mac dfont** | `.dfont` | ✔ | ✔ | `macbinary.c` | `ff_ttfdfont`, `ff_otfdfont`, `ff_otfciddfont`; resource fork stored in the data fork. |
| **Mac family (FOND set)** | `.dfont`/`.bin` | — | ✔ | `macbinary.c:1796 WriteMacFamily` | Generates one resource file containing all open styles of the family. Dialog gate at `savefontdlg.c:2077-2081`. |
| **BinHex resources** | `.hqx` | ✔ | — | `macbinary.c:3004 IsResourceInHex` | 6-bit decode with `0x90` run-length (`outchr:2984`). |
| **TrueType/OpenType Collection** | `.ttc` | ✔ | ✔ | `parsettf.c:615 PickTTFFont`, `tottf.c:6802 ttc_dump`, `:7034 WriteTTC` | See §4. |
| **OpenType (CFF)** | `.otf` | ✔ | ✔ | `parsettf.c`, `tottf.c`, `dumppfa.c` (CFF gen) | `OTTO` magic. |
| **OpenType CID** | `.otf` | ✔ | ✔ | `tottf.c:dumpcidglyphs` | `ff_otfcid`; auto-selected when `sf->cidmaster!=NULL` (`savefont.c:1130-1133`). |
| **WOFF** | `.woff` | ✔ | ✔ | `woff.c` (`_SFReadWOFF:193`, `_WriteWOFFFont:406`) | `wOFF` magic. Preserves `woffMajor`/`woffMinor`/`woffMetadata` (round-tripped through SFD keys of the same name). Per-table zlib, only kept if smaller (`compressOrNot:117`). |
| **WOFF2** | `.woff2` | ✔ | ✔ | `woff.c:630-690` + `woff2.cc` (Google lib) | Only if `FONTFORGE_CAN_USE_WOFF2`; otherwise the extension slot in `savefont_extensions[]` becomes `NULL` and **terminates the array early** (`savefont.c:88-91`). Implemented as convert-to/from-TTF then delegate. |
| **SVG font** | `.svg` | ✔ | ✔ | `svg.c` (`SFReadSVG:3682`, `WriteSVGFont:1039`, `SFReadSVGMem:3703`) | Sniffed by `<?xm` or UTF-8 BOM + `<?xm` (`splinefont.c:1124-1127`). Needs libxml. |
| **UFO 2 / UFO 3** | `.ufo`, `.ufo2`, `.ufo3` | ✔ | ✔ | `ufo.c` (4386 lines), `glif_name_hash.cpp` | Directory format; detected by `<dir>/glyphs/contents.plist` (`splinefont.c:1064-1070`). Writes `metainfo.plist`, `fontinfo.plist`, `groups.plist`, `kerning.plist`, `vkerning.plist`, `lib.plist`, `layercontents.plist`, `features.fea`, `glyphs*/*.glif`. `ttf_native_kern` toggles `sf->preferred_kerning` to force native (not class-expanded) kerning (`savefont.c:864-880`); UFO2 additionally ORs in `4` for legacy class-name style. Glyph filenames are mangled/uniquified via `ufo_name_mangle`/`ufo_name_number` (`ufo.c:1961-1964`) and cached in `sc->glif_name`. |
| **BDF** | `.bdf` | ✔ | ✔ | `fvimportbdf.c` (`SFFromBDF:2404`, `slurp_header:506`), `dumpbdf.c:401 BDFFontDump`, `bitmapchar.c` (XLFD/props) | `STAR` magic (`STARTFONT`). Output filenames get `-<pixelsize>` or `-<size>@<depth>` appended (`savefont.c:378-383`). Resolution prompt (`savefontdlg.c:999+`). |
| **PCF / PMF** | `.pcf`, `.pmf` | ✔ | — | `fvimportbdf.c:1368-1700` | Magic `\1fcp` (`splinefont.c:1143-1144`). PMF = Sun variant, encoding starts at 0x2000. Handles bit/byte/4-byte swap (`BitOrderInvert`, `TwoByteSwap`, `FourByteSwap`). |
| **TeX PK** | `*pk` | ✔ | — | `fvimportbdf.c:999-1330` | Selected by trailing `pk` (`splinefont.c:1174-1175`, `ispk==1`). |
| **TeX GF** | `*gf` | ✔ | — | `fvimportbdf.c:761-998` | `ispk==3`. |
| **Metafont source** | `.mf` | ✔ | — | `autotrace.c:744 SFFromMF` | Runs external `mf` then autotraces the GF output. Requires `mf` + `autotrace`/`potrace` binaries. |
| **Ikarus** | `.ik` | ✔ | — | `ikarus.c:599 SFReadIkarus` | Detected by `IK` at byte offset 98 plus `ch3==0 && ch4==55` (`splinefont.c:1145-1149`). |
| **PDF (extract embedded fonts)** | `.pdf` | ✔ | — | `parsepdf.c:2105/2175` | `%PDF` magic. Also used for single-glyph page-graphics import. |
| **Windows FNT** | `.fnt` | ✔ | ✔ | `winfonts.c:364 SFReadWinFON`, `:647 FNTFontDump` | One file per strike; `-<size>.fnt` naming. |
| **Windows FON** | `.fon` | ✔ | ✔ | `winfonts.c:757 FONFontDump` | NE executable wrapper around multiple FNTs. |
| **Palm OS bitmap** | `.pdb` | ✔ | ✔ | `palmfonts.c:346 SFReadPalmPdb`, `:609 WritePalmBitmaps` | |
| **NFNT / FOND Mac bitmap** | `.bin` / `.dfont` | ✔ | ✔ | `macbinary.c:1723 WriteMacBitmaps`, `:2653 SearchBitmapResources` | Dialog warns NFNT is obsolete under OS X (`savefontdlg.c:1439`). `bf_nfntdfont` is commented out of the enum but still occupies a slot in the **Mac** `bitmapextensions[]` (`savefont.c:96`) — that Mac array has 12 entries against an 11-member enum, so indices from `bf_fon` onward are shifted on macOS. |
| **X11 bitmap-only sfnt** | `.otb` | ✔ (as sfnt) | ✔ | `tottf.c:6130`, `:5794-5816` (`otbbitmaps`) | `bf_otb` forces `applemode=msbitmaps=false`, `onlybitmaps=true`, emits `EBDT`/`EBLC` plus a **dummy `loca`** (`dummyloca`) and `dumpnoglyphs`. `_DoSave` uses `old_psotb_flags` when combining OTB with a PostScript outline format (`savefont.c:806`). |
| **Apple bitmap-only sfnt** | `.dfont` | ✔ | ✔ | `parsettfbmf.c`, `tottf.c:5524-5543` | `bf_sfnt_dfont` → `bdat`/`bloc`, no `hhea`/`hmtx` (`tottf.c:5792,5822`). |
| **"faked" MS bitmap-only sfnt** | `.ttf` | ✔ | ✔ | `tottf.c:5439-5452` | `bf_sfnt_ms` → `EBDT`/`EBLC`/`EBSC` + real space glyphs. |
| **PS Type 3 bitmap** | `.pt3` | ✔ | ✔ | `dumppfa.c:2711 PSBitmapDump`, `bitmapchar.c`, `fvimportbdf.c:2422 SFCheckPSBitmap` | Refuses greymaps (`savefont.c:369-372`). |
| **AFM / AMFM** | `.afm`, `.amfm` | ✔ (companion on PS load, `CheckAfmOfPostScript`) | ✔ | `splinesaveafm.c`, `savefont.c:99` | `ps_flag_afm`, `ps_flag_afmwithmarks` (mark-to-base composites). |
| **PFM** | `.pfm` | — | ✔ | `savefont.c:294 WritePfmFile` | `ps_flag_pfm`; skipped for CID. |
| **TFM + ENC** | `.tfm`, `.enc` | ✔ (merge kern) | ✔ | `savefont.c:174 WriteTfmFile` | `ps_flag_tfm`. |
| **OFM + CFG (Omega)** | `.ofm`, `.cfg` | — | ✔ | `savefont.c:238 WriteOfmFile` | `ttf_flag_ofm` / `fm_flag_ofm`; 16-bit TFM. |
| **Glyph map** | `.g2n` | — | ✔ | `tottf.c:6049,6233` | `ttf_flag_glyphmap`: gid↔glyphname listing. |
| **FONTLOG** | `FONTLOG.txt` | ✔ (SFD key) | ✔ (append) | `savefont.c:320 WriteFontLog` | `ps_flag_outputfontlog`. |
| **Archives** | `.zip .tar .tgz .tbz .tbz2 .tar.gz .tar.bz2` | ✔ | — | `splinefont.c:639-864` | See §5. |

### Single-glyph export (`cvexport.c`) / import (`cvimages.c`)

| Op | Formats | Entry points |
|---|---|---|
| Export outline | EPS (`_ExportEPS:108`, with a `%%BeginPreview` greymap block), XFig (`ExportFig:506`), SVG (`ExportSVG:394` → `svg.c:_ExportSVG:1057`), GLIF (`ExportGlif:407`, v2 or v3), PDF (`_ExportPDF:183`), Raph's plate (`_ExportPlate:338`) | `ScriptExport:751` dispatches on integer `format` 0..5 |
| Export bitmap | XBM, BMP, PNG (`BCExportXBM:639`, `ExportImage:535`); dialog also offers XPM and "C FontForge" | `format-6` indexes `BITMAP_FORMAT_START` in `fontforgeexe/cvexportdlg.c:382-398` |
| Import glyph | image, EPS, PDF page graphics, SVG, GLIF, plate, XFig (+ "Template" bulk variants for image/EPS/PDF/SVG/GLIF) | `enum fvformats` `baseviews.h:283-291`; `FVImportImages:994`, `FVImportImageTemplate:1040`; Python importers at `fv_pythonbase=0x100` |
| Import bitmap strikes | BDF, TTF, PCF/pmf, Mac bitmap, Win FON, Palm | `fvimportbdf.c:2277 FVImportBDF`, `:2369 FVImportMult` |
| Autotrace | via external `autotrace` or `potrace` | `autotrace.c:235/329 _SCAutoTrace`, chosen by `strstrmatch(prog,"potrace")` (`:254`) |

Export/import knobs are tiny: `ExportParams { use_transform }` and `ImportParams { correct_direction, simplify, clip, erasers, scale, dimensions, accuracy_target, default_joinlimit }` (`sd.h:186-207`).

---

## 2. sfnt generation flags (what the Save dialog exposes)

`enum ttf_flags` (`splinefont_enums.h:87-110`), set from `enum fm_flags` by `fmflag2ttfflag()`:

| Flag | Dialog label (`savefontdlg.c`) | Effect |
|---|---|---|
| `ttf_flag_shortps` (1<<0) | "PS Glyph Names" (inverted) | Forces `post` format 3.0 instead of 2.0 — no glyph names (`tottf.c:4081`). OTF/OTF-CID always use 3.0. |
| `ttf_flag_nohints` (1<<1) | "TrueType Hints" (inverted) | Drops per-glyph instructions (`tottf.c:876`). |
| `ttf_flag_applemode` (1<<2) | "Apple" | `morx`/`feat`/`kern`/`opbd`/`lcar`/`prop`, `bdat`/`bloc`, different `post` naming rules, unscaled composites. |
| `ttf_flag_otmode` (1<<5) | "OpenType" | `GSUB`/`GPOS`/`GDEF`, `EBDT`/`EBLC`. Both may be set (dual-mode) or neither. Default `old_sfnt_flags = ttf_flag_otmode` (`savefont.c:60`). |
| `ttf_flag_oldkern` (1<<9) | "Old style 'kern'" | Emit legacy `kern` alongside GPOS. Mutually exclusive with Apple mode. |
| `ttf_flag_oldkernmappedonly` (1<<29) | "Windows-compatible 'kern'" | Exclude unencoded / non-BMP glyphs from the old `kern` table. |
| `ttf_flag_symbol` (1<<14) | "TrueType (Symbol)" format | See Symbol row above. |
| `ttf_flag_dummyDSIG` (1<<15) | "Dummy 'DSIG'" | 8-byte empty DSIG (makes some validators happy). In a TTC the DSIG lives once at collection level and header version becomes `0x00020000` (`tottf.c:6810-6830`). |
| `ttf_flag_glyphmap` (1<<6) | "Output Glyph Map" | writes `.g2n`. |
| `ttf_flag_TeXtable` (1<<7) | "TeX Table" | `'TeX '` private table: `fontparams`/`htdp`/`itlc` subtables (`ttfspecial.c:1786 tex_dump`). |
| `ttf_flag_ofm` (1<<8) | "Output OFM & CFG" | Omega metrics. |
| `ttf_flag_noFFTMtable` (1<<10) | "FFTM Table" (inverted) | `'FFTM'` = 3×(FontForge source date, font creation, font modification) as 1904 epoch pairs; 7 longs (`ttfspecial.c:2228 ttf_fftm_dump`). |
| `ttf_flag_pfed_comments` (1<<3) | "Save Comments" | PfEd `fcmt` (font comment), `flog` (FONTLOG), `cmnt` (per-glyph), `cvtc` (per-cvt-entry comments). |
| `ttf_flag_pfed_colors` (1<<4) | "Save Colors" | PfEd `colr`. |
| `ttf_flag_pfed_lookupnames` (1<<11) | "Lookup Names" | PfEd `GPOS`/`GSUB` subtables holding lookup+subtable names. Only written when `at->opentypemode` (`ttfspecial.c:845`). |
| `ttf_flag_pfed_guides` (1<<12) | "Save Guides" | PfEd `guid`: sorted/dedup'd h & v guideline positions with names, plus a full spline dump. |
| `ttf_flag_pfed_layers` (1<<13) | "Save Layers" | PfEd `layr`: background/extra layers, incl. **spiro** contours, delta-encoded with a 2-bit magnitude modifier (`pfed_mod_type`/`pfed_write_data`, `ttfspecial.c:351-537`). |
| `ttf_native_kern` (1<<16) | "Prefer native kerning" | Mostly UFO. |
| `ttf_flag_nomacnames` (1<<30) | "No Mac Names" | Suppress duplicate Mac-platform `name` records (`tottf.c:3987`). |
| `ttf_flag_no_outlines` (1<<19) | — (HarfBuzz internal) | skip `glyf`/`CFF `. |

PostScript-side: `ps_flag_noflex`, `ps_flag_nohints`, `ps_flag_afm`, `ps_flag_pfm`, `ps_flag_tfm`, `ps_flag_round` ("round coordinates to integers"), `ps_flag_nocffsugar`, `ps_flag_identitycidmap`, `ps_flag_afmwithmarks`, `ps_flag_noseac`, `ps_flag_outputfontlog`. Defaults: `old_ps_flags = ps_flag_afm|ps_flag_round` (`savefont.c:61`), `old_psotb_flags = ps_flag_afm`.

The `'PfEd'` table itself is version `0x00010000` with a `(tag,offset)` subtable directory (`ttfspecial.c:856-870`); tags defined in `PfEd.h:152-165`. `'BDF '` (`ttf_bdf_dump`, `ttfspecial.c:2014`) stores per-strike XLFD properties, keyed by pixelsize, with a shared string pool and de-duplicated COMMENT lines — this is what makes BDF→TTF→BDF round-tripping preserve X properties.

---

## 3. SFD format

### Overall shape

```
SplineFontDB: 3.2
FontName: …            FullName: …       FamilyName: …     Weight: …
UComments: "utf7"      FontLog: "utf7"   Version: …        FONDName: …
ItalicAngle/UnderlinePosition/UnderlineWidth/Ascent/Descent/InvalidEm
sfntRevision: 0x…      woffMajor/woffMinor/woffMetadata
UFOAscent/UFODescent
LayerCount: n
Layer: <i> <order2> "name" <isbackground> ["ufo_path"]
StrokedFont/MultiLayer/HasVMetrics/XUID/UniqueID/FSType/…
OS2Version, OS2*, Panose, PfmFamily, TTFWeight, TTFWidth, MacStyle, StyleMap
Lookup: <type> <flags> <store_in_afm> "name" { "sub1" "sub2" … } [ 'feat' ( 'scpt' < 'lang' … > ) … ]
KernClass2:/VKernClass2:, ContextPos2:/ContextSub2:/ChainPos2:/ChainSub2:/ReverseChain2: … EndFPST
MacIndic2:/MacContext2:/MacLigature2:/MacSimple2:/MacInsert2:/MacKern2: … EndASM
MacFeat:/MacName:/MacSetting:, Justify:/JstfLang:/JstfPrio:/JstfExtender:
TtTable: prep|fpgm  … EndTTInstrs         (disassembled instructions)
ShortTable: cvt |maxp <n> … EndShort      (decimal shorts, cvt entries may carry "names")
TtfTable: <tag> <len> … EndTtf            (ASCII-85 binary blob)
LangName: <langid> "n0" "n1" … , GaspTable:, DesignSize:, OtfFeatName:
MATH:<Name>: <v> [devtab] …               (per math_constants_descriptor)
PickledData:/PickledDataWithLists: "…"    (Python persistent data)
Encoding: <name/…>   |  Registry:/Ordering:/Supplement:/CIDVersion: for CID
UnicodeInterp: …   NameList: …   RemapN:/Remap:
DisplaySize/DisplayLayer/AntiAlias/FitToEm/ExtremaBound/WidthSeparation/WinInfo
BeginPrivate: n … EndPrivate
GridOrder2: / Grid  <splineset>  EndSplineSet
TeXData: <type> <designsize> <22 params>
AnchorClass2: "name" "subtable" …
BeginSubFonts: <cnt> <max> … EndSubFonts     (CID)
BeginMMFonts: …                              (MM)
BeginChars: <enccount> <realcnt>
   … StartChar/EndChar blocks …
EndChars
BitmapFont: … EndBitmapFont   (repeated per strike)
EndSplineFont | EndSubSplineFont
```
(`sfd.cpp:2180-2860`; `SFD_DumpSplineFontMetadata` at `:2163`.)

### Per-glyph block (`SFDDumpChar`, `sfd.cpp:1373-1629`)

```
StartChar: <name>            (or UTF-7 quoted string if non-ASCII)
Encoding: <enc> <unicode> <gid>
AltUni2: <uni>.<varsel>.<fid> …
GlifName: <name>
Width: n     [VWidth: n]     [GlyphClass: n] [UnlinkRmOvrlpSave: n] [InSpiro: n]
Flags: HMWOI                 (H=changedsincelasthinted M=manualhints W=widthset O=open-view I=instrs-stale)
TeX: <height> <depth>        ItalicCorrection:/TopAccentHorizontal:/IsExtendedShape:/GlyphVariants*/GlyphComposition*
HStem: <start> <width>[G][<b e b e>] …
VStem: …
DStem2: <lx> <ly> <rx> <ry> <ux> <uy> [<…>] …
CounterMasks: <n> <hex> …
TtInstrs: … EndTTInstrs
AnchorPoint: "class" <x> <y> <type> <ligidx> [devtabs | ttfpt]
LayerCount: n
Fore | Back | Layer: <i> [multilayer: dofill dostroke fillfirst #fillcol op #strokecol op width join cap [trans] [dashes]]
  Image:/Image2: …
  SplineSet
     … path lines …
  EndSplineSet
  Refer: <gid> <uni> S|N <a> <b> <c> <d> <e> <f> <flags> [<basept> <refpt> [O]]
  Guideline: "name" <x> <y> <angle> <color> <flags>
  Validated: <mask>
Kerns2:/VKerns2: <gid> <off> "subtable" [devtab] …
Position2:/PairPos2:/Substitution2:/AlternateSubs2:/MultipleSubs2:/Ligature2:/LCarets2: …
Comment: "utf7"   Decomposition: "utf7"   Colour: <hex>
TileMargin:/TileBounds:
EndChar
```

`Refer:` flags = `use_my_metrics | round_translation_to_grid<<1 | point_match<<2` (`sfd.cpp:1096-1097`).
`AnchorPoint` type strings: `entry|exit|mark|basechar|baselig|basemark` (`sfd.cpp:669-673`).
Device tables serialize as `{first-last c0,c1,…}` (`SFDDumpDeviceTable:630`).

### SplineSet path syntax (`SFDDumpSplineSet` `sfd.cpp:517-628`, parser `SFDGetSplineSet:3932`)

Reverse-Polish, one point per line, `%g` (or `%.12g` in double builds):

```
x y m <flags>[,<ttfindex>,<nextcpindex>][x<hexmask>]
 x y l <flags>…
 x1 y1 x2 y2 x y c <flags>…
```
* `m` = moveto (starts a new contour), `l` = lineto, `c` = curveto with `(prev-point's nextcp), (this point's prevcp), (this point)`.
* `l` is only emitted when `sp->prev->islinear && sp->noprevcp` — a linear segment that still carries control points is written as `c`.
* Contours are implicitly **closed**; the writer emits the first point again at the end. An open contour is signalled by `0x400` on the last point, and the reader calls `SFDCloseCheck` unless that bit is set (`sfd.cpp:4006, 4096`).

Point flags bitfield (`sfd.cpp:104-113`, written at `:557-574`, read at `:4048-4062`):

| Bit | Name | Meaning |
|---|---|---|
| `0x3` | `SFD_PTFLAG_TYPE_MASK` | `sp->pointtype` (0 curve, 1 corner, 2 tangent, 3 hvcurve) |
| `0x4` | `IS_SELECTED` | only when `SaveEditorState` |
| `0x8` | `NEXTCP_IS_DEFAULT` | `nextcpdef` |
| `0x10` | `PREVCP_IS_DEFAULT` | `prevcpdef` |
| `0x20` / `0x40` | `ROUND_IN_X` / `ROUND_IN_Y` | |
| `0x80` | `INTERPOLATE` | on write means `ttfindex==0xffff` (implied/interpolated TT point); on read sets `ttfindex=0xffff`, else auto-increments |
| `0x100` | `INTERPOLATE_NEVER` | `dontinterpolate` |
| `0x200` | `PREV_EXTREMA_MARKED_ACCEPTABLE` | on `sp->prev` spline |
| `0x400` | `FORCE_OPEN_PATH` | contour is open |

Trailer forms after the flags integer:
* quadratic layers: `,<ttfindex>,<nextcpindex>` (either may be `-1` = 0xffff; `,,` means 0xfffe "unset").
* cubic layers: `x<hexdigits>` = a `HintMask` (`SFDDumpHintMask:498`, two hex digits per byte, trailing zero bytes trimmed).

Optional per-contour trailers before `EndSplineSet`:
```
NamedP: "point name"     (immediately after a point line)
  Spiro
    x y <ty>             ty is the spiro point type char
  EndSpiro
  Named: "contour name"
  PathFlags: <1 = clip path>
  PathStart: <start_offset>
```

Bitmap strikes:
```
BitmapFont: <pixelsize> <glyphcnt> <ascent> <descent> <depth> [foundry]
BDFStartProperties: n
  <NAME> <type> <int|"string">
BDFEndProperties
Resolution: n
BDFChar: <gid> <enc> <width> <xmin> <xmax> <ymin> <ymax> [vwidth]
<ASCII-85 rows>
BDFRefChar: <gid> <refgid> <xoff> <yoff> S|N
EndBitmapFont
```

Strings that are not pure ASCII are written **UTF-7-ish** by `SFDDumpUTF7Str` (`sfd.cpp:209`) and read by `SFDReadUTF7Str` — a custom base64 escape; binary blobs use a custom ASCII-85 (`SFDEnc85`). Any Rust re-implementation needs both codecs verbatim.

---

## 4. Unknown-table preservation on TTF/OTF round trip

`struct ttf_table { uint32_t tag; uint32_t len, maxlen; uint8_t *data; struct ttf_table *next; FILE *temp; }` — `splinefont.h:1161-1167`. `SplineFont` holds **two** lists (`splinefont.h:1272-1273`):

* `sf->ttf_tables` — semantically owned instruction/limit tables: `fpgm`, `prep`, `cvt `, `maxp`. Always kept and re-emitted (`dumpstoredtable`, `tottf.c:5271`, called at `:5839-5841`).
* `sf->ttf_tab_saved` — opaque blobs kept byte-for-byte, re-emitted with `dumpsavedtable` (`tottf.c:5292`) and appended to the table directory at `tottf.c:5713-5719` (with an `IError` if `MAX_TAB` is exceeded).

The split happens on load at `parsettf.c:6181-6200`. Which tables land in `ttf_tab_saved` is controlled by:
* the `PreserveTables` preference — a comma-separated list of 4-char tags parsed into `info->savetab` by `ParseSaveTablesPref` (`parsettf.c:696-726`; pref registered in `fontforgeexe/prefs.c:328`), **or**
* `openflags & of_all_tables` (`splinefont_enums.h:120`), which preserves *every* table not otherwise understood (`parsettf.c:700-703, 1227-1231`).

Everything else that FontForge does not parse is **dropped** with a `LogError("  Ignoring '%c%c%c%c' %s")`, using the 100-entry name table `stdtables[]` at `parsettf.c:925-1011` for the human-readable description. Notably in the *ignored-unless-preserved* bucket: `COLR`/`CPAL` (absent even from `stdtables`), `CBDT`/`CBLC`, `sbix`, `CFF2`, `STAT`, `HVAR`/`VVAR`/`MVAR`, `SVG `, `meta`, `ltag`, `trak`, `Zapf`, `fdsc`, `just`, `hsty`, `gcid`, `ankr`, `MERG`, `ELUA`, and all SIL Graphite tables (`Silf`, `Glat`, `Gloc`, `Sill`, `Feat`). **FontForge has no reader or writer for any of these**; they survive only as opaque `ttf_tab_saved` blobs.

Everything else is *regenerated from scratch* every save: `head`, `hhea`, `maxp`, `OS/2`, `hmtx`, `vhea`/`vmtx`, `cmap`, `name`, `post`, `glyf`/`loca` or `CFF `, `kern`, `gasp`, `GDEF`/`GSUB`/`GPOS`, `morx`/`feat`/`prop`/`lcar`/`opbd`/`bsln`, `BASE`, `JSTF`, `MATH`, `VORG`, `LTSH`, `VDMX`, `hdmx`, `PCLT`, `DSIG`, `EBDT`/`EBLC`/`EBSC` or `bdat`/`bloc`, `fvar`/`gvar`/`avar`/`cvar`, plus the FontForge-private `PfEd`, `TeX `, `FFTM`, `BDF `.

In SFD, both lists are serialized by `SFDDumpTtfTable` (`sfd.cpp:1002-1053`, driven from `:2627-2630`) using three encodings: `prep`/`fpgm` as disassembled instruction text, `cvt `/`maxp` (even length) as decimal shorts with optional per-entry names, everything else as ASCII-85 under `TtfTable:`. On read, the tag decides which list it goes back into (`sfd.cpp:3756/3806/3846`).

Table ordering on output follows Adobe's recommendation, implemented as an `orderingval` switch in `initTables` (`tottf.c:5860-5905`), differing between TrueType and CFF-flavoured output. Duplicate tables are shared by pointing two directory entries at the same offset (`tottf.c:5924-5930`).

---

## 5. TTC / collections

**Reading** — `readttfheader` (`parsettf.c:1013-1027`) sees `ttcf`, sets `info->is_ttc` and calls `PickTTFFont` (`parsettf.c:615-675`). Selection order: (1) if the filename carried a `(name)` suffix, match against each subfont's `name`-table name; (2) if that fails, try to parse the suffix as a decimal index; (3) otherwise `ff_choose()` a list; (4) `no_windowing_ui` ⇒ index 0. `cnt==1` short-circuits. The parenthesized subfont name is extracted by `SFSubfontnameStart` (`splinefont.c:914-929`), which scans for a *balanced* `(`…`)` that ends the string. `of_all_glyphs_in_ttc` (`splinefont_enums.h:117`) makes the loader keep every glyph in the file rather than only those reachable from the chosen font (`parsettf.c:2282`).

**Writing** — `WriteTTC` (`tottf.c:7034`). `ttc_flag_cff` forces `ff_otf` output, otherwise `ff_ttf` (line 7047). Two strategies:
1. *Merged* (`ttc_flag_trymerge`, and only when `bf==bf_none`): `ttc_prep` builds a shared glyph set, `ttc_dump` (`:6802`) writes one `glyf`/`loca` (or one `CFF `), one `hmtx`/`hhea`/`maxp`/`FFTM`, and per-font `head`/`name`/`OS/2`/`cmap`/`post`/`gasp`/`fpgm`/`prep`/`cvt `/`PfEd`/`TeX ` (`ttc_perfonttables:6700`). Identical per-font tables are then coalesced by byte comparison (`tablefilematch:6772`, `checkdupstoredtable:6684`). Header space is pre-reserved at `8+16*MAX_TAB` bytes per font.
2. *Brute force* fallback: generate each font independently into a temp file with `_WriteTTFFont(..., flags & ~ttf_flag_dummyDSIG, ...)` and concatenate them 4-byte aligned via `CopySFNTAndFixup`.

In both cases the collection header is `ttcf` + version `0x00010000`, or `0x00020000` with a trailing collection-level dummy `DSIG` when `ttf_flag_dummyDSIG` is set.

`ff_ttc` never flows through `_DoSave`; the dialog calls `WriteTTC` directly (`fontforgeexe/savefontdlg.c:1387, 1566-1570`) with `ttc_flag_trymerge`/`ttc_flag_cff` from the "Merge tables across fonts" / "As CFF fonts" checkboxes.

---

## 6. Open flow (`_ReadSplineFont`, `splinefont.c:933-1277`)

1. **`file://` URIs** are unescaped and treated as paths (`:952-956`). There is no HTTP/URL fetching left in the tree.
2. Trailing `/` on a directory font (`foo.ufo/`) is stripped (`:959-962`).
3. **Subfont suffix**: `SFSubfontnameStart` splits `path(Name)` into `strippedname` + `chosenname` (`:968-973`).
4. **Archive**: extension matched against `archivers[]` (`:639-654`: `.tar .tgz .tar.gz .tar.bz2 .tbz2 .tbz .zip`; zip uses `unzip` on POSIX, `tar` on Win10+). The archive is expanded into a temp dir; `ArchiveParseTOC` (`:662`) picks the member. If exactly one file → use it. If everything sits under one `*.ufo`/`*.ufo2`/`*.ufo3`/`*.sfdir` directory → extract the whole thing (`doall`). Otherwise prompt, with a default chosen by priority: `.sfd`(40) > `.otf/.ttf/.ttc`(30) > `.pfb/.pfa/.cff/.cid`(20) > `.svg`(10).
5. **Compression**: `ff_compression_type()` recognizes `.gz .bz2/.bz .lzma .xz .Z` (`inc/ffprocess.h:112-118`); `Decompress()` (`:866`) shells out to a temp file. When a `FILE*` (not a path) was supplied, `ForceFileToHaveName` (`:877`) first spills it to a temp file with the right extension.
6. **Directory fonts**: `<dir>/glyphs/contents.plist` → UFO; `<dir>/font.props` → SFDir (`:1064-1078`).
7. **Magic sniffing** on the first 7 bytes plus bytes 98–99 (`:1082-1152`). Detection markers, in order: `00 01 00 00` / `OTTO` / `true` / `ttcf` → sfnt; `wOFF` → WOFF; `wOF2` → WOFF2; `%!` or `80 01` → PostScript; `%PDF` → PDF; `01 00 04` → bare CFF; `<?xm` (or BOM + `<?xm`) → SVG; `Spli` → SFD; `STAR` → BDF; `\1fcp` → PCF; `IK` at offset 98 with `ch3==0 && ch4==55` → Ikarus. The comment at `:1150` notes Mac resource files have no usable magic.
8. **Extension fallback** (`:1156-1211`) for anything the sniffer missed, guarded by the `checked` character so a format isn't retried: `.sfd`/`.sfd~`, `.ttf/.ttc/.gai/.otf/.otb`, `.svg`, `.ufo/.ufo2/.ufo3`, `.bdf`, `*pk`, `*gf`, `.pcf/.pmf`, `.bin/.hqx/.dfont`, `.fon/.fnt`, `.pdb`, `.pfa/.pfb/.pf3/.cid/.gsf/.pt3/.ps`, `.cff`, `.mf`, `.pdf`, `.ik`. **The final `else` is `SFReadMacBinary`** — i.e. an unrecognized file is assumed to be a Mac resource fork.
9. Post-load: `origname` is rebuilt as `path(chosenname)` for collections; archived fonts get `origname=NULL, isnew=true` so "Save" won't overwrite the archive; `fstype==2` (Restricted License) prompts unless the font came from SFD or `of_fstypepermitted` was passed (`:1265-1274`).

`LoadSplineFont` (`:1283`) additionally retries the bare name with each of `.sfd .pfa .pfb .ttf .otf .ps .cid .bin .dfont` (and uppercase variants).

---

## 7. Test fixtures (`fontforge/tests/`)

46 files in `tests/fonts/` plus `tests/fonts/bitmap_refs/`. Bundled fixtures and what they exercise (from `tests/CMakeLists.txt:52-160` and `tests/README`):

**SFD fixtures** — `Ambrosia.sfd` / `AmbrosiaBold.sfd` / `AmbrosiaItalic.sfd` (Type 3 read/gen, Mac family generation, FEA export, SFNT names, italicize, zipped-SFD open, `file://`); `Caliban.sfd` (reverse-chaining, multi-format generate, bitmap export); `CaslonMM.sfd` (Multiple Master → pfa round trip); `DejaVuSerif.sfd` (WOFF validation, string conversion, glyph 'i'); `NimbusLGCUni-Regular.sfd` (Unicode encodings); `VKern.sfd` (kerning); `CMAPEncTest.sfd` (TrueType cmap encoding); `SFDBitmapParsing.sfd` (malformed-SFD parsing, test934); `DataURI.sfd` (SVG + PT3 with data URIs); `courB18.sfd` (bitmap export); `FormalScript.sfd`, `dvng10a-crash.sfd` (contextual chaining crashes); `OverlapBugs.sfd`, `QuadOverlapBugs.sfd`, `SplineOverlapBug1.sfd`, `SimplifyBugs.sfd`, `AddExtremumTest.sfd`, `AddExtremaTest2.sfd`, `QuadraticConversionBug.sfd`, `NumberPoints.sfd`, `DirectionTest.sfd`, `StrokeTests.sfd`, `Hinting.sfd`, `AHBugs.sfd`, `BoxesAndBounds.sfd`, `OmittedCharBugs.sfd`.

**Binary fixtures** — `NotoSerifTibetan-Regular.ttf` (glyph names from an unusual `post` table, test1013); `test1012.otf` (GPOS kerning with r2l lookup flag); `CantarellMin.woff2` (WOFF2 round trip, test132/test927); `n019003l.pfb`, `cmbsy10.pfb`, `ecrm1000.pfb`, `feta20.pfb` (Type 1 read, merge fonts, merge kern); `ayn+meem.init.svg` (SVG import, zero-length stroke).

**FEA fixtures** — `test133.fea` (glyph-class name characters), `test134.fea` (short-hand contextual multiple substitution), `test135.fea` (GDEF glyph classes), `test136.fea` (mark filtering set reuse), `test1011.fea` (r2l kerning), `test1017.fea` (invalid substitutions).

**Reference images** — `tests/fonts/bitmap_refs/`: `caliban_*` and `court18_*` in `bmp`/`png`/`xbm` at depths 1/2/4/8, consumed by `test_bitmap_export.py`.

**Not bundled** (skipped unless the user drops them in, `tests/README`): `Zapfino-4.0d3.dfont`, `Zapfino-4.1d6.dfont` (dfont + `morx`), `HanSans-Bold.otf` (CID OTF), `MunhwaGothic-Bold` (non-OTF CID), `mingliu.ttc` / `NotoSans-Regular.ttc` / `LucidaGrande.ttc` (TTC open, subfont open — test130, test0101, test929), `GillSans.dfont`, `CourierNew.ttf`, `merged.cff`, `MinionPro-Regular.otf`, `nuvo-medium-woff-demo.woff` (WOFF major/minor/metadata, test1002).

Format-relevant Python tests worth mirroring in a Rust suite: `test_generate.py` and `test001.pe` (multi-format generate), `test926.py`/`test927.py` (WOFF/WOFF2), `test1014.py` (**PfEd layers round tripping** — the only direct test of private-table preservation), `test_persistent_payload.py` (Python persistent data through SFD), `test_sfd_suppl_plane.py` (astral-plane names in SFD), `test930.py`/`test931.py`/`test932.py` (point/contour/layer fidelity and selection round trip — effectively SFD splineset fidelity tests), `test934.py` (malformed SFD), `test_open_zip.py` (archive open path), `test_export.py` / `test_bitmap_export.py` (single-glyph export).

---

## 8. Gotchas worth carrying into a re-implementation

* **Extension sniffing on save is ad-hoc string matching** (`savefont.c:1089-1160`): `.ttf.bin`/`.suit` → MacBin, `.bin`/`.res` → Type1 MacBin, `.sym.ttf` → symbol TTF, `.cid.cff` → CFF CID, `.cid.t42` → Type42 CID, `.mm.pfa`/`.mm.pfb` → MM, `.mult` or a `%s` in a `.pfa`/`.pfb` name → multiple-PS, `.otf.dfont` overrides the `.dfont` match. Several of these are `strmatch(end-len,…)` without a length guard on short filenames.
* `_DoSave` picks the flag word by *enum ordering*, not by capability: `oldformatstate<=ff_cffcid` ⇒ PS flags, else sfnt flags (`savefont.c:800-807`). Any reordering of `enum fontformat` silently breaks this — hence the warning comment at `splinefont_enums.h:30-33`.
* Apple vs OpenType is a 2-bit space, and *both unset* is a legal state meaning "neither table family" (`savefont.c:1057-1070`).
* `.notdef` glyphs with outlines are silently omitted, with a confirm dialog (`savefontdlg.c:1343`).
* Generating an OTF from a font whose `OS/2` version is ≤1 is refused outright (`savefontdlg.c:1382`).
* Writing a `cvt ` table that FontForge itself invented for `.notdef` is undone after generation so autoinstructing still sees a blank slate (`tottf.c:5936-5957`).
* SFD's `SplineSet` closes contours implicitly — a naive round trip that omits the `0x400` bit will silently close open paths.
* `Kerns2:`/`VKerns2:`, `Position2:`…`LCarets2:`, `KernClass2:`, `AnchorClass2:`, `ChainSub2:` etc. all carry a `2` suffix marking the post-`sfd_version>=2` lookup-based model; `sfd1.c` exists purely to migrate the pre-2 flat SLI/feature model (`SFD_AssignLookups`, `sfd1.c:661`).
