<!-- Research report generated 2026-09-03 by an Explore agent reading fontforge/ (FontForge git dc05099f, 2026-08-10). Paths are relative to fontforge/. -->

# FontForge UI Feature / Command Inventory

Extracted from `fontforge/fontforgeexe/`, the FontForge reference checkout in this repository. Shortcut column shows the FontForge default hotkey where one exists (defaults live in `doc/sphinx/ui/misc/HotKeys.rst`; the menu arrays themselves ship `H_("...|No Shortcut")` because bindings are user-rebindable via `gdraw/hotkeys.c` + `~/.config/fontforge/hotkeys`).

---

## 1. Source map

| Area | File |
|---|---|
| Font View window + all its menus | `fontview.c` (menu arrays `fllist edlist ellist enlist vwlist htlist mtlist cdlist mmlist wnmenu mblist`, popup `fvpopupmenu`) |
| Outline Glyph View | `charview.c` (`fllist edlist ptlist spiroptlist ellist htlist vwlist mtlist mmlist mblist`), `charview_private.h`, `cv_mids.h` |
| Tool palette / layer palette | `cvpalettes.c`; tool implementations `cvpointer.c cvfreehand.c cvknife.c cvruler.c cvhand.c cvshapes.c cvaddpoints.c cvtranstools.c` |
| Metrics View | `metricsview.c`, `mv_mids.h` |
| Bitmap View | `bitmapview.c`, `bv_mids.h`, `bitmapdlg.cpp` |
| Font Info | `fontinfo.c` |
| Glyph Info | `charinfo.c` |
| Lookups/subtables, kerning, anchors | `lookupui.c`, `kernclass.c`, `anchorsaway.c`, `contextchain.c`, `statemachine.c`, `showatt.c` |
| Find Problems / Validate | `problems.cpp` |
| Transform family | `transform.c`, `nonlineartransui.c`, `cvstroke.c`, `effectsui.c`, `tilepath.c`, `simplifydlg.c`, `scstylesui.c` |
| Hinting / instructing | `cvhints.c`, `ttfinstrsui.c`, `cvdebug.c`, `deltaui.c`, `cvgridfit.c`, `freetypeui.c`, `histograms.cpp` |
| Encoding / groups / namelists | `encodingui.c`, `groupsdlg.c` |
| Print / Display specimen | `displayfonts.c`, `sftextfield.c`, `wordlistparser.c` |
| Search & replace | `searchview.c` |
| MM / CID | `mmdlg.c` |
| MATH | `math.c` |
| Preferences | `prefs.c` |
| Scripting / Python | `scriptingdlg.c`, `pythonui.c`, `usermenu.c`, `pluginui.cpp` |

---

## 2. Font View window (`fontview.c`)

Menu bar: `mblist` = File, Edit, Element, Tools, Hints, Encoding, View, Metrics, CID, MM, Window, Help.

### 2.1 File (`fllist`)

| Command | What it does | Function | Key |
|---|---|---|---|
| Font > New | Create empty font | `MenuNew` | Ctrl-N |
| Hangul > New Composition… / Modify Composition… / Build Syllables | Korean jamo composition rules; batch-build syllable glyphs | `MenuNewComposition`, `FVMenuModifyComposition`, `FVMenuBuildSyllables` | |
| Open | Open font/SFD/archive | `FVMenuOpen` | Ctrl-O |
| Recent | Dynamic recent-file list | `MenuRecentBuild` | |
| Close | Close this font window | `FVMenuClose` | Ctrl-Shift-Q |
| Save / Save as… / Save All | Save SFD; save-as; save every open font | `FVMenuSave`, `FVMenuSaveAs`, `MenuSaveAll` | Ctrl-S / Ctrl-Shift-S |
| Generate Fonts… | Export binary font (TTF/OTF/CFF/Type1/bitmap…) | `FVMenuGenerate` | Ctrl-Shift-G |
| Generate Mac Family… | Emit a Mac family suitcase from several open fonts | `FVMenuGenerateFamily` | Alt-Ctrl-G |
| Generate TTC… | Emit a TrueType Collection | `FVMenuGenerateTTC` | |
| Import… | Import image/EPS/SVG/bitmap strike into glyphs | `FVMenuImport` | Ctrl-Shift-I |
| Merge Feature Info… | Merge AFM/FEA/kern data into font | `FVMenuMergeKern` | Alt-Ctrl-Shift-K |
| Revert File / Revert To Backup / Revert Glyph | Discard changes at file / backup / glyph level | `FVMenuRevert`, `FVMenuRevertBackup`, `FVMenuRevertGlyph` | Ctrl-Shift-R / Ctrl-Alt-R |
| Clear Special Data | Strip retained binary tables etc. | `FVMenuClearSpecialData` | |
| Print… | Font specimen printing | `FVMenuPrint` | Ctrl-P |
| Execute Script… | Run FF-script or Python | `FVMenuExecute` | Ctrl-. |
| Script Menu | User script slots | `MenuScriptsBuild` | Alt-Ctrl-1..9 |
| Preferences… / Appearance Editor… / Configure Plugins… | Prefs dialog; GResource theme editor; plugin manager | `MenuPrefs`, `MenuXRes`, `MenuPlug` | |
| Quit | Exit | `FVMenuExit` | Ctrl-Q |

### 2.2 Edit (`edlist`)

| Command | What it does | Function | Key |
|---|---|---|---|
| Undo / Redo / Undo Fontlevel | Per-glyph undo stacks; font-level undo (`sfundo.c`) | `FVMenuUndo`, `FVMenuRedo`, `FVMenuUndoFontLevel` | Ctrl-Z / Ctrl-Y |
| Cut / Copy / Paste / Paste Into / Paste After / Clear | Glyph-level clipboard ops | `FVMenuCut/Copy/Paste/PasteInto/PasteAfter/Clear` | Ctrl-X/C/V, Ctrl-Shift-V |
| Copy Reference | Copy as component reference | `FVMenuCopyRef` | Ctrl-G |
| Copy Lookup Data | Copy PST/kern/anchor data attached to glyph | `FVMenuCopyLookupData` | Alt-Ctrl-C |
| Copy Width / VWidth / LBearing / RBearing | Copy just one metric | `FVMenuCopyWidth` | Ctrl-W |
| Same Glyph As | Mark glyphs as identical (dup) | `FVMenuSameGlyphAs` | |
| Clear Background / Copy Fg To Bg / Copy Layer To Layer | Layer transfer (`layer2layer.c`) | `FVMenuClearBackground`, `FVMenuCopyFgBg`, `FVMenuCopyL2L` | Ctrl-Shift-C |
| Join | Join open contour endpoints | `FVMenuJoin` | Ctrl-Shift-J |
| Select > All / Invert / Deselect All | Selection basics | `FVMenuSelectAll/InvertSelection/DeselectAll` | Ctrl-A / Ctrl-Esc / Esc |
| Select > by Color / by Wildcard… / by Script… | Filter selection | `FVMenuSelectColor`, `FVMenuSelectByName`, `FVMenuSelectByScript` | |
| Select > Worth Outputting / only References / only Splines / both / Whitespace / Changed / Hinting Needed / Autohintable | Predicate selects (Shift merges, Ctrl restricts) | `FVMenuSelectWorthOutputting`, `FVMenuGlyphsRefs`, `FVMenuGlyphsSplines`, `FVMenuGlyphsBoth`, `FVMenuGlyphsWhite`, `FVMenuSelectChanged`, `FVMenuSelectHintingNeeded`, `FVMenuSelectAutohintable` | |
| Select > By Lookup Subtable… | Select glyphs participating in a lookup | `FVMenuSelectByPST` | |
| Find / Replace… | Outline pattern search (`searchview.c`) | `FVMenuFindRpl` | Alt-Ctrl-F |
| Replace with Reference | Replace matching outlines by references | `FVMenuReplaceWithRef` | Alt-Ctrl-Shift-F |
| Correct References | Fix stale reference point-matching | `FVMenuCorrectRefs` | |
| Unlink Reference | Inline components | `FVMenuUnlinkRef` | Ctrl-U |
| Copy From > All Fonts / Displayed Font / Glyph Metadata / TrueType Instructions | Clipboard scope flags | `FVMenuCopyFrom` | |
| Remove Undoes | Free undo memory | `FVMenuRemoveUndoes` | |

### 2.3 Element (`ellist`)

| Command | What it does | Function | Key |
|---|---|---|---|
| Font Info… | 21-tab font metadata editor | `FVMenuFontInfo` | Ctrl-Shift-F |
| Glyph Info… | 15-tab per-glyph editor | `FVMenuCharInfo` | Alt-Ctrl-Shift-I |
| Other Info > MATH Info… | OpenType MATH table editor (`math.c`) | `FVMenuMATHInfo` | |
| Other Info > BDF Info… | Bitmap strike properties (`bdfinfo.c`) | `FVMenuBDFInfo` | |
| Other Info > Horizontal/Vertical Baselines… | 'BASE' table editor (`basedlg.c`) | `FVMenuBaseHoriz`, `FVMenuBaseVert` | |
| Other Info > Justification… | 'JSTF' table editor (`justifydlg.c`) | `FVMenuJustify` | |
| Other Info > Show Dependent > References / Substitutions | List glyphs depending on selection | `FVMenuShowDependentRefs`, `FVMenuShowDependentSubs` | Alt-Ctrl-I |
| Other Info > Mass Glyph Rename… | Bulk rename by suffix/namelist (`lookupui.c`) | `FVMenuMassRename` | |
| Other Info > Set Color | Assign fontview cell colour | `FVMenuSelectColor` | |
| Validation > Find Problems… | 40+ configurable outline/hint/ATT checks | `FVMenuFindProblems` | Ctrl-E |
| Validation > Validate… | Output-blocking error list w/ auto-fix | `FVMenuValidate` | |
| Validation > Set Extremum Bound… | Threshold for missing-extrema check | `FVMenuSetExtremumBound` | |
| Bitmap Strikes Available… / Regenerate Bitmap Glyphs… / Remove Bitmap Glyphs… | Bitmap strike management | `FVMenuBitmaps` | Ctrl-Shift-B / Ctrl-B |
| Style > Change Weight… | Embolden LCG/CJK/auto (`scstylesui.c`) | `FVMenuEmbolden` | |
| Style > Italic… | Full italic transformation w/ per-glyph substitution rules | `FVMenuItalic` | |
| Style > Oblique… | Simple slant | `FVMenuOblique` | |
| Style > Condense/Extend… | Independent stem/counter scaling | `FVMenuCondense` | |
| Style > Change X-Height… | Rescale x-height, preserve stems | `FVMenuChangeXHeight` | |
| Style > Change Glyph… | Generic stem/counter/serif transform | `FVMenuChangeGlyph` | |
| Style > Add Small Capitals… | Build smcp/c2sc glyphs + lookup | `FVMenuSmallCaps` | |
| Style > Add Subscripts/Superscripts… | Build subs/sups/sinf/dnom/numr | `FVMenuSubSup` | |
| Style > Inline… / Outline… / Shadow… / Wireframe… | Decorative effects (`effectsui.c`) | `FVMenuInline`, `FVMenuOutline`, `FVMenuShadow`, `FVMenuWireframe` | |
| Transformations > Transform… | Move/scale/rotate/flip/skew stack (`transform.c`) | `FVMenuTransform` | Ctrl-\ |
| Transformations > Point of View Projection… | 3-D perspective projection | `FVMenuPOV` | |
| Transformations > Non Linear Transform… | x/y expression warp (`nonlineartransui.c`) | `FVMenuNLTransform` | |
| Expand Stroke… | Nib/calligraphic/polygonal stroke expansion (`cvstroke.c`) | `FVMenuStroke` | Ctrl-Shift-E |
| Tile Path… / Tile Pattern… | Repeat a tile along a path / fill a rect (`tilepath.c`) | `FVMenuTilePath`, `FVMenuPatternTile` | |
| Overlap > Remove Overlap / Intersect / Find Intersections | Boolean ops | `FVMenuOverlap` | Ctrl-Shift-O |
| Simplify > Simplify / Simplify More… / Cleanup Glyph / Canonical Start Point / Canonical Contours | Point reduction + normalisation (`simplifydlg.c`) | `FVMenuSimplify`, `FVMenuSimplifyMore`, `FVMenuCleanup`, `FVMenuCanonicalStart`, `FVMenuCanonicalContours` | Ctrl-Shift-M / Alt-Ctrl-Shift-M |
| Add Extrema / Add Points Of Inflection | Insert required on-curve points | `FVMenuAddExtrema`, `FVMenuAddInflections` | Ctrl-Shift-X / Ctrl-Shift-Y |
| Balance / Harmonize | Even out handles / G2-smooth junctions | `FVMenuBalance`, `FVMenuHarmonize` | Ctrl-Shift-P / Ctrl-Shift-Z |
| Round > To Int / To Hundredths / Cluster | Coordinate rounding & clustering | `FVMenuRound2Int`, `FVMenuRound2Hundredths`, `FVMenuCluster` | Ctrl-Shift-_ |
| Autotrace | Run autotrace/potrace on background image | `FVMenuAutotrace` | Ctrl-Shift-T |
| Correct Direction | Fix winding order | `FVMenuCorrectDir` | Ctrl-Shift-D |
| Build > Build Accented Glyph / Composite Glyph / Duplicate Glyph | Auto-compose from Unicode decomposition | `FVMenuBuildAccent`, `FVMenuBuildComposite`, `FVMenuBuildDuplicate` | Ctrl-Shift-A |
| Merge Fonts… | Merge another font's glyphs+kerning (`fvfontsdlg.c`) | `FVMenuMergeFonts` | |
| Interpolate Fonts… | Blend two compatible fonts by percentage | `FVMenuInterpFonts` | |
| Compare Fonts… | Diff outlines/hints/names/bitmaps/GSUB/GPOS (`showatt.c`) | `FVMenuCompareFonts` | |
| Compare Layers… | Diff two layers with error bound (`layer2layer.c`) | `FVMenuCompareL2L` | |

### 2.4 Tools

Dynamic; built by `FVGetToolsSubmenu()` / `fvpy_tllistcheck` — Python plugin menu entries registered via `fontforge.registerMenuItem` (`pythonui.c`, `usermenu.c`).

### 2.5 Hints (`htlist`)

| Command | What it does | Function | Key |
|---|---|---|---|
| AutoHint | Generate PS stem hints | `FVMenuAutoHint` | Ctrl-Shift-H |
| Hint Substitution Pts | Compute hintmask positions | `FVMenuAutoHintSubs` | |
| Auto Counter Hint | Counter-control hints (`hintmask`/`cntrmask`) | `FVMenuAutoCounter` | |
| Don't AutoHint | Lock glyph against autohinting | `FVMenuDontAutoHint` | |
| AutoInstr | Generate TrueType instructions | `FVMenuAutoInstr` | Ctrl-T |
| Edit Instructions… | TT assembly editor (`ttfinstrsui.c`) | `FVMenuEditInstrs` | |
| Edit 'fpgm' / 'prep' / 'maxp' / 'cvt ' | Edit font programs / CVT table | `FVMenuEditTable` | |
| Remove Instr Tables | Drop fpgm/prep/cvt | `FVMenuRmInstrTables` | |
| Suggest Deltas… | DELTA-instruction hint suggestions (`deltaui.c`) | `FVMenuDeltas` | |
| Clear Hints / Clear Instructions | Remove hints / TT code | `FVMenuClearHints`, `FVMenuClearInstrs` | |
| Histograms > HStem / VStem / BlueValues | Stem-width histograms (`histograms.cpp`) | `FVMenuHistograms` | |

### 2.6 Encoding (`enlist`)

| Command | What it does | Function |
|---|---|---|
| Reencode > … | Switch display encoding (built by `FVEncodingMenuBuild`; ISO 8859-1..16, Unicode BMP/Full/by-plane, Mac, Symbol, Big5/HKSCS, GB2312, EUC-GB, SJIS, JIS208/212, Johab, KSC5601, Wansung, KOI8-R, Adobe Standard, TeX Base 8r, Glyph Order, Custom, CID orderings — `encodingui.c`) | `FVEncodingMenuBuild` |
| Compact (hide unused glyphs) | Hide empty encoding slots | `FVMenuCompact` |
| Force Encoding > … | Reassign glyphs to encoding slots destructively | `FVForceEncodingMenuBuild` |
| Add Encoding Slots… | Append unencoded slots | `FVMenuAddUnencoded` |
| Remove Unused Slots | Trim tail | `FVMenuRemoveUnused` |
| Detach Glyphs / Detach & Remove Glyphs… | Unencode (keep) / unencode + delete | `FVMenuDetachGlyphs`, `FVMenuDetachAndRemoveGlyphs` |
| Add Encoding Name… / Load Encoding… / Make From Font… / Remove Encoding… | Custom encoding management | `FVMenuAddEncodingName`, `FVMenuLoadEncoding`, `FVMenuMakeFromFont`, `FVMenuRemoveEncoding` |
| Display By Groups… / Define Groups… | Group-based fontview browsing (`groupsdlg.c`) | `FVMenuDisplayByGroups`, `FVMenuDefineGroups` |
| Save Namelist of Font… / Load Namelist… / Rename Glyphs… / Create Named Glyphs… | Glyph-naming conventions | `FVMenuMakeNamelist`, `FVMenuLoadNamelist`, `FVMenuRenameByNamelist`, `FVMenuNameGlyphs` |

### 2.7 View (`vwlist`)

| Command | What it does | Function | Key |
|---|---|---|---|
| Next / Prev Glyph, Next / Prev Defined Glyph | Move selection | `FVMenuChangeChar` | Ctrl-] / Ctrl-[ |
| Goto | Jump to glyph by name/codepoint (`gotodlg.c`) | `FVMenuGotoChar` | Ctrl-Shift-> |
| Layers > … | Choose active layer shown in cells | `lylist` |
| Show ATT | Browse all GSUB/GPOS/morx tables (`showatt.c`) | `FVMenuShowAtt` | |
| Display Substitutions… | Show cells rendered through a chosen lookup | `FVMenuDisplaySubs` | |
| Combinations > Kern Pairs / Anchored Pairs / Ligatures | Browse pairs & ligatures (`combinations.c`) | `FVMenuKernPairs`, `FVMenuLigatures` | |
| Label Glyph By > Glyph Image / Name / Unicode / Encoding Hex | Cell label mode | `gllist` | |
| Show H. Metrics… / Show V. Metrics… | Overlay baseline/x-height/ascent lines in cells | `FVMenuShowMetrics` | |
| 32x8 / 16x4 / 8x2 cell window | Grid dimensions | `FVMenuWSize` | Ctrl-Shift-% / ^ / * |
| 24 / 36 / 48 / 72 / 96 / 128 pixel outline | Cell pixel size | `FVMenuSize` | Ctrl-2/3/4/7/9 |
| Anti Alias / Fit to font bounding box | Rendering options | `FVMenuSize` | Ctrl-5 / Ctrl-6 |
| Bitmap Magnification… | Zoom bitmap strike display | `FVMenuBitmapMag` | |

### 2.8 Metrics (`mtlist`)

| Command | What it does | Function | Key |
|---|---|---|---|
| New Metrics Window | Open metrics/kerning window | `FVMenuOpenMetrics` | Ctrl-K |
| Center in Width / Thirds in Width | Auto-place glyph in its advance | `FVMenuCenter` | Ctrl-C (menu) |
| Set Width… / Set LBearing… / Set RBearing… / Set Both Bearings… / Set Vertical Advance… | Absolute / increment / scale metric setting (`fvmetricsdlg.c`) | `FVMenuSetWidth` | Ctrl-Shift-L / Ctrl-L / Ctrl-R |
| Auto Width… | Spacing algorithm (`autowidth2dlg.c`: Separation, Min/Max Bearing, Height, Loop Count) | `FVMenuAutoWidth` | Ctrl-Shift-W |
| Kern By Classes… | Class-kerning matrix editor (`kernclass.c`) | `FVMenuKernByClasses` | |
| Remove All Kern Pairs | Delete horizontal kerning | `FVMenuRemoveKern` | |
| Kern Pair Closeup… | Two-glyph kerning magnifier | `FVMenuKPCloseup` | |
| VKern By Classes… / VKern From HKern / Remove All VKern Pairs | Vertical kerning | `FVMenuVKernByClasses`, `FVMenuVKernFromHKern`, `FVMenuRemoveVKern` | |

### 2.9 CID (`cdlist`)

Convert to CID (`FVMenuConvert2CID`), Convert By CMap (`FVMenuConvertByCMap`), Flatten (`FVMenuFlatten`), FlattenByCMap (`FVMenuFlattenByCMap`), Insert Font… (`FVMenuInsertFont`), Insert Blank (`FVMenuInsertBlank`), Remove Font (`FVMenuRemoveFontFromCID`), Change Supplement… (`FVMenuChangeSupplement`), CID Font Info… (`FVMenuCIDFontInfo`).

### 2.10 MM (`mmlist`) — `mmdlg.c`

Create MM… (`FVMenuCreateMM`), MM Validity Check (`FVMenuMMValid`), MM Info… (`FVMenuMMInfo`), Blend to New Font… (`FVMenuBlendToNew`), MM Change Default Weights… (`FVMenuChangeMMBlend`). Dialog fields: Axis 1–4, Axis Type, Axis Range (Begin/Default/End), Design Settings, Contribution of each master design, Convert Design Vector Function, Normalize Design Vector — Adobe MM / Apple distortable variants.

### 2.11 Window (`wnmenu`) / Help

New Outline Window (`FVMenuOpenOutline`, Ctrl-H), New Bitmap Window (`FVMenuOpenBitmap`, Ctrl-J), New Metrics Window (Ctrl-K), Warnings (`_MenuWarnings`), plus dynamic open-window list (`windowmenu.c`). Help = Help/Index/About/License (`helplist`, F1).

### 2.12 Right-click cell popup (`fvpopupmenu`)

New Outline Window, Cut, Copy, Copy Reference, Copy Width, Paste, Clear, Copy Fg To Bg, Unlink Reference, Glyph Info…, Transform…, Expand Stroke…, To Int, Correct Direction, AutoHint, Center in Width, Set Width…, Set Vertical Advance…

---

## 3. Outline Glyph View (`charview.c`)

Menu bar: File, Edit, Point, Element, Tools, Hints, View, Metrics, MM, Window, Help (`mblist` / `mblist_nomm`).

### 3.1 File extras beyond Font View
Close Tab (`CVMenuCloseTab`), Export… (`CVMenuExport`, `cvexportdlg.c`: EPS/PDF/SVG/Glif/BMP/PNG/XBM/XPM/XFig/plate), Import… (`CVMenuImport`, `cvimportdlg.c` with Import Parameters: Correct Direction, Handle Erasers, Simplify Stroke, Join Limit, Scale to fit, Infer glyph width, Use Clip-paths, Accuracy Target), Load Word List… (`CVAddWordList`), Revert Glyph.

### 3.2 Edit (`edlist`) — glyph-specific
Undo/Redo/Cut/Copy/Copy Reference/Copy Lookup Data/Copy Width/LBearing/RBearing/Paste; **Chop** (`CVClear`); Clear Background; **Merge** (`CVMerge`, Ctrl-M) and **Merge to Line** (`CVMergeToLine`); **Elide** (`CVElide`); **Join** (`CVJoin`); Copy Fg To Bg; Copy Layer To Layer… (`CVMenuCopyL2L`); Copy Grid Fit (`CVMenuCopyGridFit`); Unlink Reference; Remove Undoes.

Select submenu (`sllist`): Select All (Alt-Ctrl-A), Invert, Deselect All, First Point (`CV_MID_FirstPt`), First Point Next Contour (Alt-Ctrl-.), Next / Prev Point (Ctrl-Shift-} / {), Next / Prev Control Point, Points on Selected Contours (Alt-Ctrl-,), Point At… (Ctrl-,), Select All Points & Refs, Select Open Contours, Select Anchors, Width, VWidth, Points Affected by HM — `CVSelectAll`, `CVMenuNextPrevPt`, `CVMenuNextPrevCPt`, `CVMenuSelectContours`, `CVMenuSelectPointAt`, `CVSelectOpenContours`, `CVSelectWidth`, `CVSelectVWidth`, `CVSelectHM`.

### 3.3 Point (`ptlist` / `spiroptlist`)

| Command | What it does | Function | Key |
|---|---|---|---|
| Curve / HVCurve / Corner / Tangent | Change on-curve point type | `CVMenuPointType` | Ctrl-2 / 3 / 4 |
| Make First | Set contour start point | `CVMenuMakeFirst` | Ctrl-1 |
| Can Be / Can't Be Interpolated | TrueType implied-point flag | `CVMenuImplicit` | |
| Center Between Control Points | Recentre on-curve point | `CVMenuCenterCP` | |
| Add Anchor | Attach anchor point to glyph | `CVMenuAddAnchor` | Ctrl-0 |
| Acceptable Extrema | Whitelist a non-extremum | `CVMenuAcceptableExtrema` | |
| Make Line / Make Arc | Convert segment | `CVMenuMakeLine` | |
| Insert Point On Spline At… | Insert at t / coordinate | `CVMenuInsertPt` | |
| Name Point / Name Contour | Named points & contours (UFO) | `CVMenuNamePoint`, `CVMenuNameContour` | |
| Make Clip Path | Type3 clipping contour | `CVMenuClipPath` | |
| Tools > … | Tool palette mirror | `cvtoollist` | |
| *(spiro mode)* G4 Curve / G2 Curve / Corner / Left Constraint / Right Constraint / Make First | Spiro control point types | `CVMenuPointType`, `CVMenuSpiroMakeFirst` | |

### 3.4 Element (`ellist`) — additions over Font View
Get Info… (`CVMenuGetInfo`, Ctrl-I → point/reference/anchor/image info, `cvgetinfo.cpp`); Align > Align Points (Ctrl-Shift-@) / Space Points (Ctrl-Shift-#) / Space Regions… / Make Parallel… (`CVMenuConstrain`, `CVMenuMakeParallel`, `alignment.c`); Order > First / Earlier / Later / Last (`CVMenuOrder`, reference & path z-order); Overlap adds **Exclude**; Check Self-Intersection (`CVMenuCheckSelf`), Glyph Self-Intersects flag; Clockwise / Counter Clockwise / Correct Direction / Reverse Direction (`CVMenuDir`, `CVMenuCorrectDir`, `CVMenuReverseDir`); Insert Text Outlines… (`CVMenuInsertText`, `displayfonts.c`).

### 3.5 Hints (`htlist`)
AutoHint, Hint Substitution Pts, Auto Counter Hint, Don't AutoHint, AutoInstr, Edit Instructions…, **Debug…** (`CVMenuDebug`, TT bytecode debugger `cvdebug.c`), Suggest Deltas…, Clear HStem / VStem / DStem, Clear Instructions, Add HHint / VHint / DHint (`CVMenuAddHint`), Create HHint… / VHint… (`CVMenuCreateHint`), **Review Hints…** (`CVMenuReviewHints`, Alt-Ctrl-H — `cvhints.c` with Next/Prev, Base, Size, Move Points, Regenerate Hint Substitution Points).

### 3.6 View (`vwlist`)

Fit (Ctrl-F) / Zoom out (Alt-Ctrl--) / Zoom in (Alt-Ctrl-Shift-+) — `CVMenuScale`; Display Compositions…; Next/Prev/Next Defined/Prev Defined/Former Glyph (`CVMenuChangeChar`); Goto; Find In Font View (Ctrl-Shift-<).

- **Number Points** (`nplist`): None / TrueType / SVG / Positions — `CVMenuNumberPoints`.
- **Grid Fit** (`gflist`): Show Grid Fit… (`CVMenuShowGridFit`, `cvgridfit.c`: DPI, Pointsize X/Y, Mono/Anti-Aliased, Debug fpgm/prep), Show Grid Fit (Live Update), Bigger/Smaller Point Size, Anti Alias (Ctrl-=), Off.
- **Show** (`swlist`): Points (Ctrl-D), Control Points (Always), Control Point Info, Extrema, Points of Inflection, Almost Horizontal/Vertical Lines, Almost H/V Curves, Side Bearings, Reference Names, Fill, Preview, Dragging Comparison Outline, Palettes > Tools / Layers / Docked Palettes, Glyph Tabs, Rulers, Horizontal Hints, Vertical Hints, Diagonal Hints, BlueValues, FamilyBlues, Anchors, Debug Raster Changes, Horizontal Metric Lines, Vertical Metric Lines, Snap Outlines to Pixel Grid — `CVMenuShowHide`, `CVMenuShowHints`, `CVMenuMarkExtrema`, `CVMenuShowAlmostHV`, `CVMenuShowSideBearings`, `CVMenuShowRefNames`, `CVMenuFill`, `CVMenuPreview`, `CVMenuPaletteShow`, `CVMenuShowTabs`, `CVMenuShowHideRulers`, `CVMenuSnapOutlines`.
- **Combinations** (`cblist`): Kern Pairs, Anchored Pairs, **Anchor Control…** (`anchorsaway.c`), Anchor Glyph at Point, Ligatures.
- Next/Previous Line in Word List (`CVMenuNextLineInWordList`) — word-list driven proofing (`wordlistparser.c`).

### 3.7 Metrics (`mtlist`)
New Metrics Window, Center in Width, Thirds in Width, Set Width/LBearing/RBearing/Both Bearings/Vertical Advance, Kern By Classes…, VKern By Classes…, VKern From HKern, Remove Kern Pairs, Remove VKern Pairs, Kern Pair Closeup… (`CVMenuKPCloseup`).

### 3.8 Tool palette (`cvpalettes.c`, `cvtoollist`)

| Tool | enum | Modifier behaviour |
|---|---|---|
| Pointer | `cvt_pointer` | Shift = constrain H/V/italic; Alt = pick control points first |
| Magnify | `cvt_magnify` | Alt = minify; Shift = square marquee |
| Freehand | `cvt_freehand` | Shift = constrain 45° (`cvfreehand.c`) |
| Scroll (hand) | `cvt_hand` | `cvhand.c` |
| Knife | `cvt_knife` | Cuts splines in two (`cvknife.c`) |
| Ruler / Measure | `cvt_ruler` | Distance & angle readout, snapping (`cvruler.c`) |
| Pen | `cvt_pen` | Add point + drag control points (`cvaddpoints.c`) |
| Activate/Deactivate Spiro | `cvt_spiro` | Toggle Spiro clothoid mode |
| Curve / HVCurve / Corner / Tangent | `cvt_curve` `cvt_hvcurve` `cvt_corner` `cvt_tangent` | Point-adding tools |
| Spiro G4 / G2 / Corner / Left / Right | `cvt_spirog4` `cvt_spirog2` `cvt_spirocorner` `cvt_spiroleft` `cvt_spiroright` | Spiro point tools |
| Scale / Rotate / Flip / Skew / 3D Rotate / Perspective | `cvt_scale` `cvt_rotate` `cvt_flip` `cvt_skew` `cvt_3d_rotate` `cvt_perspective` | Direct-manipulation transforms (`cvtranstools.c`) |
| Rectangle / Ellipse / Polygon / Star | `cvt_rect` `cvt_elipse` `cvt_poly` `cvt_star` | Shape tools; Shift = square/circle; double-click sets corner radius / vertex count (`cvshapes.c`) |

**Layer palette** (`cvpalettes.c` `LayerMenu`): Fore / Back / Guide radio + per-layer visibility, and context menu New Layer, Del Layer, Shift Contents To First / Up / Down / To Last, Make Foreground/Background, Make Cubic/Quadratic, Fill, Show Cubic Column, Show Fore/Back Column.

**Canvas context menu** (`CVToolsPopup`): point-type items plus Get Info…, Open Reference, Add Anchor.

---

## 4. Metrics View (`metricsview.c`)

Menus: File, Edit, Element, View, Metrics, Window, Help. Grid table with editable rows: Name, Width, LBearing, RBearing, Kern; plus BBearing. Window Type (`tylist`): Kerning only / Advance Width only / Both (`MVMenuWindowType`).

| Menu > Command | What it does | Function |
|---|---|---|
| File > Load Word List… | Load proofing strings | `MVMenuAddWordList` |
| Edit > Undo/Redo/Cut/Copy/Copy Reference/Copy Width/VWidth/LBearing/RBearing/Paste/Clear/Join/Select All/Deselect All/Unlink Reference | Standard glyph editing on the selected cell | `MVUndo` … `MVUnlinkRef` |
| Element > Font Info… / Glyph Info… / Show Dependent / Find Problems… | Same as elsewhere | `MVMenuFontInfo`, `MVMenuCharInfo`, `MVMenuShowDependents`, `MVMenuFindProblems` |
| Element > Bitmap Strikes Available… / Regenerate Bitmap Glyphs… | Strike management | `MVMenuBitmaps` |
| Element > Transform… / Tile Path… / Remove Overlap / Simplify / Add Extrema / Add Points Of Inflection / To Int / Effects (Inline/Outline/Shadow/Wireframe) / Autotrace / Correct Direction / Build | Outline ops on selected glyph | `MVMenuTransform`, `MVMenuTilePath`, `MVMenuOverlap`, `MVMenuSimplify`, `MVMenuAddExtrema`, `MVMenuAddInflections`, `MVMenuRound2Int`, `MVMenuInline`…, `MVMenuAutotrace`, `MVMenuCorrectDir`, `MVMenuBuildAccent` |
| View > Zoom in/out | Scale display | `MVMenuScale` |
| View > Insert Glyph After… / Before… / Replace Glyph… | Edit the sample string | `MVMenuInsertChar`, `MVMenuChangeChar` |
| View > Next/Prev/Next Defined/Prev Defined Glyph, Find In Font View | Navigation | `MVMenuChangeChar`, `MVMenuFindInFontView` |
| View > Layers / Combinations (Kern Pairs, Anchored Pairs, Ligatures) | Layer + pair browsing | `lylist`, `cblist` |
| View > Show Grid > Show / Partial / Hide when Moving / Hide | Baseline+bearing grid | `MVMenuShowGrid` |
| View > Anti Alias / Render using Hinting / Vertical / Outline | Rendering mode; vertical typesetting | `MVMenuAA`, `MVMenuRenderUsingHinting`, `MVMenuVertical`, `MVMenuShowBitmap` |
| View > Size set from Window / Set Point Size / Bigger / Smaller Point Size | Display size | `MVMenuSizeWindow`, `MVMenuPointSize`, `MVMenuChangePointSize` |
| View > Next / Previous Line in Word List | Step proofing strings | `MVMenuNextLineInWordList` |
| Metrics > Center in Width / Thirds / Set Width / LBearing / RBearing / Both Bearings / Vertical Advance | Metric edits | `MVMenuCenter`, `MVMenuSetWidth` |
| Metrics > Window Type / Kern By Classes… / VKern By Classes… / VKern From HKern / Remove Kern Pairs / Remove VKern Pairs / Kern Pair Closeup… | Kerning | `MVMenuWindowType`, `MVMenuKernByClasses`, `MVMenuVKernByClasses`, `MVMenuVKernFromHKern`, `MVMenuRemoveKern`, `MVMenuRemoveVKern`, `MVMenuKPCloseup` |

The metrics view has a script/language/feature selector row driven by HarfBuzz/FF's own shaper (`DefaultShaper` pref, `prefs.c`).

---

## 5. Bitmap View (`bitmapview.c`)

Menus: File, Edit, Element, View, Metrics, Window, Help.

| Menu > Command | What it does | Function |
|---|---|---|
| File > Export… / Import… | Bitmap in/out | `BVMenuExport`, `BVMenuImport` |
| Edit > Undo/Redo/Cut/Copy/Copy Reference/Paste/Clear/Select All/Remove Undoes/Unlink Reference | Bitmap clipboard | `BVUndo` … `BVUnlinkRef` |
| Element > Font Info… / Glyph Info… / BDF Info… | Metadata | `BVMenuFontInfo`, `BVMenuGetInfo`, `BVMenuBDFInfo` |
| Element > Bitmap Strikes Available… / Regenerate Bitmap Glyphs… / Remove This Glyph | Strike ops | `BVMenuBitmaps`, `BVMenuRmGlyph` |
| Element > Transformations > Flip Horizontally / Vertically / Skew… | Bitmap transforms | `BVMenuRotateInvoked` (`bvt_fliph/flipv/skew`) |
| View > Fit / Zoom out / Zoom in / Next / Prev / Next Defined / Prev Defined / Goto / Find In Font View / Bigger / Smaller Pixel Size | Navigation & zoom | `BVMenuScale`, `BVMenuChangeChar`, `BVMenuGotoChar`, `BVMenuChangePixelSize` |
| View > Palettes > Tools / Layers / Shades / Docked Palettes | Palette visibility | `BVMenuPaletteShow`, `BVMenuPalettesDock` |
| Metrics > Set Width… / Set Vertical Width… | Bitmap advance | `BVMenuSetWidth` |

Bitmap tool palette (`cvpalettes.c` `bvpopups`, enum `bvtools`): Pointer, Magnify (Minify with alt), Set/Clear Pixels (pencil), Draw a Line, Shift Entire Bitmap, Scroll Bitmap; also `bvt_eyedropper`, `bvt_setwidth`, `bvt_setvwidth`, `bvt_rect`, `bvt_filledrect`, `bvt_elipse`, `bvt_filledelipse`. Shades palette for greymap depths.

---

## 6. Font Info dialog (`fontinfo.c`) — 21 tabs

| Tab | Contents |
|---|---|
| **PS Names** | Fontname, Family Name, Name For Humans, Weight, Version, sfnt Revision, Base Filename (+"Same as Fontname"), Copyright |
| **General** | Ascent, Descent, Em Size, Scale Outlines, Italic Angle (+Guess), Underline Position/Height, Has Vertical Metrics, Interpretation, Name List |
| **Layers** | Font Type (Outline / Type3 Multi Layered / Stroked + Stroke Width), All layers cubic / quadratic / Mixed, Guidelines quadratic, per-layer table (name, cubic/quad, background flag) |
| **PS UID** | Use XUID / XUID value, Use UniqueID / UniqueID value |
| **PS Private** | Private dict entries with Guess and Histogram buttons (BlueValues, OtherBlues, FamilyBlues, StdHW/StdVW, StemSnapH/V, BlueFuzz, BlueScale, BlueShift, ForceBold, ExpansionFactor, LanguageGroup…) |
| **OS/2** (sub-tabs `vaspects`) | **Misc.** (Weight Class, Width Class, PFM Family, Embeddable/fsType, No Subsetting, Only Embed Bitmaps, Vendor ID, IBM Family, OS/2 Version, Style Map, Weight-Width-Slope Only); **Metrics** (Win Ascent/Descent Offset ± Is Offset, Really use Typo metrics, Typo Ascent/Descent/Line Gap, HHead Ascent/Descent/Line Gap, VHead Column Spacing, Capital Height, X Height); **Sub/Super** (Subscript/Superscript X, Y, Size, Strikeout Size/Pos); **Panose** (10 Panose digits); **Charsets** (Unicode & codepage ranges) |
| **TTF Names** | Full 'name' table matrix: platform/language × name-ID (Family, SubFamily, UniqueID, Fullname, Version, PostScriptName, Trademark, Manufacturer, Designer, Descriptor, Vendor URL, Designer URL, License, License URL, Preferred Family/Styles, Compatible Full, Sample Text, CID findfont Name, WWS Family/Subfamily), with "Same as PostScript Names"/"Detach", Multi-line edit, Delete |
| **StyleSet Names** | ss01–ss20 UI names ('name' IDs for feature params) |
| **Grid Fitting** | 'gasp' table rows (ppem, grid-fit, anti-alias, symmetric smoothing/gridfit), plus head flags |
| **ΤεΧ** | TeX font parameters (text/math/mathext families of dimen params) |
| **Size** | 'size' feature: design size, size range, subfamily ID, subfamily name |
| **Comment** | Free-text font comment |
| **FONTLOG** | FONTLOG.txt-style change log |
| **Mark Classes** | Named GDEF mark classes (name + glyph list) |
| **Mark Sets** | GDEF mark attachment sets |
| **OpenType Lookups** | GSUB / GPOS lookup+subtable tree — context menu: Top, Up, Down, Bottom, Sort, Add Lookup, Add Subtable, Edit Metadata, Edit Data, Delete, Merge, Save Lookup…, Add Language to Script…, Remove Language from Script…, Add 'aalt' features, Add 'DFLT' script, Revert All, Import…, Save Feature File… |
| **WOFF** | WOFF major/minor version, metadata XML |
| **Mac** | Mac style bits, Mac family/style names |
| **Mac Features** | 'feat'/'morx' feature & setting names, Enabled flag, Feature Id (`macencui.c`) |
| **Dates** | Creation / modification timestamps |
| **Unicode Ranges** | Coverage report by Unicode block |

Also reachable: **CID Font Info** (registry/ordering/supplement) and, in the same dialog family, `FontInfo` variants for MM fonts.

---

## 7. Glyph Info dialog (`charinfo.c`) — up to 15 tabs

| Tab | Contents |
|---|---|
| **Unicode** | Glyph Name, Unicode value, "Interpreted as:", OT Glyph Class, Color, Mark for Unlink+Remove Overlap before Generating, Alternate Unicode Encodings / Variation Selectors table |
| **Comment** | Free-text per-glyph note |
| **Positionings** | GPOS single-position entries (Δx, Δy, Δx_adv, Δy_adv + device tables) |
| **Pairwise Pos** | GPOS kern pairs w/ full 8-value + device-table matrix, Hide Unused Columns |
| **Substitutions** | GSUB single substitutions per lookup subtable |
| **Alt Subs** | 'aalt' alternate lists |
| **Mult Subs** | Multiple/decomposition substitutions |
| **Ligatures** | Ligature component lists per subtable |
| **Lig. Carets** | Ligature caret count/positions, Default Ligature Caret Count |
| **Components** | Read-only decomposition ("Accented glyph composed of:") |
| **Counters** | Counter mask editing ("Select hints between which counters are formed", New Counter Mask) |
| **ΤεΧ & Math** | TeX height/depth/italic correction; MATH italic correction, Top Accent horizontal position, Is Extended Shape, Math Kerning (Top Right/Top Left/Bottom Right/Bottom Left corner tables) |
| **Vert. Variants** | MATH vertical glyph variants + vertical glyph construction (extension parts: glyph, StartLen, EndLen, FullLen, Extender) |
| **Horiz. Variants** | Same for horizontal |
| **Tile Size** | Tile margins for Tile Path/Pattern use |

Buttons: Prev / Next glyph navigation, Set From Name, Set From Value, Select By ATT… → **Select By Lookup Subtable** dialog (Select Results / Merge Results / Restrict Selection).

---

## 8. Lookups, kerning, anchors, contextual (`lookupui.c`, `kernclass.c`, `anchorsaway.c`, `contextchain.c`, `statemachine.c`)

**Lookup Table Edit** (`EditLookup`, `lookupui.c`): Type (GSUB: Single, Multiple, Alternate, Ligature, Contextual, Contextual Chaining, Reverse Chaining, Mac Indic/Contextual/Insertion State Machine; GPOS: Single, Pair (kerning), Cursive, Mark-to-Base, Mark-to-Ligature, Mark-to-Mark, Contextual, Contextual Chaining, Mac Kerning State Machine), Lookup Name, feature/script/language matrix (`LK_NewFeature`, `LK_ScriptsDlg`, `LK_GTKLangsDlg` — full OT script list `scripts[]` ~200 entries and language list `languages[]` ~500 entries), flags Right To Left, Ignore Base Glyphs, Ignore Ligatures, Ignore Combining Marks, Mark Class, Mark Set. `CreateAndSortNewLookupOfType`.

**Subtable data editors** (`PSTKernDlg`): per-type matrix with sorting (Alphabetic / By Scripts / By Base Char / Unicode), Hide Unused Columns, Default New Entries to First, Populate, Remove Empty, Remove All, AutoKern Selected, Auto_Kern params (Default Separation, Min Kern, Touching, Only kern glyphs closer, Autokern new entries).

**Kern By Classes / VKern By Classes** (`kernclass.c`): two class lists (first/second), {All}/{Everything Else} pseudo-classes, matrix grid with kern offsets, Kern Offset field, Device Table Correction at display size, Clear Device Table, Clear All Device Tables, Revert Kerning, AutoKern All / Row / Column, Separation, Min Kern, Touching, Show Kerning preview, Use FreeType, Select Class Containing…, Display Size, Magnification. **Kern Pair Closeup** dialog with magnified two-glyph preview.

**Anchor Control** (`anchorsaway.c`): per-class Bases / Marks / Entries / Exits lists, live overlay preview at chosen pixel size with magnification, X/Y coordinates, device-table Corrections per pixelsize, Add Base/Mark/Entry/Exit Anchor…, Detach.

**Contextual chaining builder** (`contextchain.c`): three formats — By Glyphs, By Classes, By Coverage; Backtrack / Match / Lookahead sections; Back/Match/Ahead class lists; ordered "Apply lookup … at position" sequence list; Add/Remove Lookup; Simple vs Complex editing modes; Reverse Chaining replacement list.

**Apple state machines** (`statemachine.c`): Indic Rearrangement (ABx=>xAB … verb table), Contextual Substitution, Contextual Glyph Insertion (Current/Mark Insert lists, Insert Before Current/Marked), Contextual Kerning (up to 8 kern values); per-state transition editor with Next State, Advance To Next Glyph, Mark Current Glyph, Push Current Glyph, {Start of Text}, {Start of Line}, {End of Text}, {Deleted Glyph}, {Everything Else}.

**Show ATT** (`showatt.c`): read-only browse of GSUB/GPOS/GDEF/'BASE'/'bsln'/'morx'/'kern'/'opbd'/'lcar'/'prop' with per-lookup rule expansion. Also hosts **Font Compare** (Compare Outlines, Bitmaps, Names, Hints, HintMasks, Glyph Positioning, Glyph Substitution, Accept inexact, Add Diff Outlines to Background, Add Missing Glyphs).

---

## 9. Find Problems (`problems.cpp`) — complete check list

Tabs: **Points**, **Hints**, **CID**, **Random**, plus Refs/Bitmaps/ATT groups.

Paths/points: Non-integral coordinates; X near ±; Y near ±; Y near standard heights; Control points near horizontal/vertical; Control points beyond spline; Irrelevant control points with factor (%); Points too close; Points too far; Open paths; Intersecting paths; Edges near horizontal/vertical; Check outermost paths clockwise; Check missing extrema; More points than N.

References: Check flipped references; References with bad TrueType transformation matrices; Mixed contours and references; References with bad PostScript transformation matrices; References nested deeper than N; References with out-of-date point matching; Multiple references with use-my-metrics.

Hints: Hints controlling no points; Points near hint edges; Hint width near ±; Almost stem3 hint; Show exact *stem3; More hints than N; Overlapped hints.

ATT/features: Check for missing glyph names; Check for missing scripts in features; Check substitutions for empty chars; Check for incomplete mark-to-base subtables.

CID: Check for CIDs defined twice; Check for undefined CIDs.

Metrics/bbox: Glyph bounding box above / below / right of / left of; Check advance; Check vertical advance.

Bitmaps: Check missing bitmaps; Bitmap/outline advance mismatch.

Naming/encoding: Check multiple Unicode; Check multiple names; Check Unicode/name mismatch.

**Validate…** window error taxonomy: Open Contour, Self Intersecting, Wrong Direction, Flipped References, Missing Points at Extrema, Unknown glyph referenced in GSUB/GPOS/MATH, Too Many Points, Too Many Hints, Bad Glyph Name, Distance between adjacent points is too big, Non-integral coordinates, Contains anchor points for some but not all classes in a subtable, Duplicate name, Duplicate unicode code point, Overlapped hints in same hintmask; plus a full PS Private-dict validator (BlueValues/OtherBlues/FamilyBlues odd count, disordered, too many, too close, non-integer, zone too big for BlueScale; Missing BlueValues; Bad BlueFuzz/BlueScale/BlueShift/StdHW/StdVW/StemSnapH/StemSnapV; StemSnap does not contain StdHW/StdVW).

Fix-ups offered: Close Open Contours, Inline All References, Remove Overlap, Mark for Overlap fix before Save, Inline Flipped References, Correct Direction, Add Good Extrema, Add All Extrema, Simplify, Revalidate All/Revalidate, Open Glyph, Scroll To Glyph, "Select Glyphs With…", "Try To Fix Glyphs With…".

---

## 10. Transform-family dialogs

**Transform** (`transform.c`): stack of up to 5 ops — Move…, Rotate… (angle, Clockwise/Withershins), Scale Uniformly…, Scale… (X/Y factors), Flip…, Skew… (angle), Rotate 3D Around… (X/Y axis), Move by Ruler…, Rotate by Ruler…, Skew by Ruler…; Origin: Glyph Origin / Center of Selection / Last Press; options Transform Width Too, Transform kerning classes too, Transform simple positioning features & kern pairs, Transform All Layers, Transform Guide Layer Too, Round To Int; warns to run Add Extrema afterwards.

**Point of View Projection / Non Linear Transform** (`nonlineartransui.c`): View Point, Direction of gaze, Drawing plane tilt, Distance to drawing/projection plane, Vanishing Point; X Expr / Y Expr with `+ - * / % ^ ?:` and functions.

**Expand Stroke** (`cvstroke.c`): nib shape Circular (Elliptical) / Calligraphic (Rectangular) / Convex (Polygonal) / custom Nib; Major Axis (Width), Minor Axis (Height), Nib Angle; Line Cap Butt / Round / Square / Extend Cap; Line Join Miter / Miter Clip / Round / Bevel / Arcs / Join Limit; Arcs Clip; Remove Internal / External Contour; Add Extrema; Simplify; Accuracy Target; Dashes list. Also hosts the Type3 **Multi-layer** editor (Fill/Stroke per layer, Color, Gradient (Linear/Radial with stops), Inherited, Dashes).

**Effects** (`effectsui.c`): Inline (Outline Width, Gap), Outline (Outline Width), Shadow (Outline Width, Shadow Length, Light Angle), Wireframe.

**Simplify More** (`simplifydlg.c`): Error Limit, Allow slopes to change, Allow removal of extrema, Allow curve smoothing (Tangent threshold), Snap to horizontal/vertical, Flatten bumps on lines (Line length max, Bump Size), Don't smooth lines, Start contours at extrema, Set as Default.

**Tile Path / Tile Pattern** (`tilepath.c`): Isolated / First / Medial / Final tiles; Tile vs Scale vs Scale & Tile; Left / Center / Right placement; Include Whitespace below Tile; Pattern Size (Width/Height), X/Y Repeat Count.

**Styles** (`scstylesui.c`): Change Weight (LCG / CJK / Auto / Custom, Embolden by, Serif height & fuzz, Counters, Top/Bottom hint zones, Cleanup Self Intersect); Italic (angle, per-glyph substitution rules, Correct for Italic Angle, Compress/Extend); Condense/Extend (Horizontal Counter Scale/Add, Horizontal Stem Height Scale/Add, Left/Right Side Bearing); Change XHeight (Current/Desired x-height, Serif height); Change Glyph (Vertical Counters for CJK / Vertical Mapping for Latin-Greek-Cyrillic, Horizontal counters, stem heights); Small Caps (Glyph Extension, Feature Tag, symbols too); Subscript/Superscript (Glyph Extension, Feature Tag, Height, Vertical Offset, Stem Height, Denominators/Numerators).

---

## 11. Hinting & instructing dialogs

- **Review Hints** (`cvhints.c`): Prev/Next hint, Base, Size, Move Points, Remove, Regenerate Hint Substitution Points. **Create HStem/VStem Hint**: Base, Size, Create.
- **Edit Instructions** (`ttfinstrsui.c`): TrueType assembler/disassembler with per-opcode tooltips for the full instruction set (PUSHB/PUSHW/NPUSHB/NPUSHW, MDAP/MIAP/MDRP/MIRP, IUP, SHP/SHC/SHZ/SHPIX, DELTAP1-3, DELTAC1-3, FDEF/IDEF/CALL/LOOPCALL, IF/ELSE/EIF, arithmetic/logical ops, SVTCA/SPVTCA/SFVTCA, ROUND/RTG/RTHG/RUTG/RDTG/ROFF, SCANCTRL/SCANTYPE, etc.), Change Length, Comment.
- **TrueType Debugger** (`cvdebug.c`): Step into / Step over (Next) / Step out of current function / Continue / Exit Debugger; panes Registers, Stack, Storage, Cvt, Points (Current / Original / Grid / Em Units / Transformed / Raw), Raster, Twilight, Gloss (`cvdgloss.c` plain-English instruction narration); watch points on points/CVT/storage (`freetypeui.c`: Hit Watch Point, Watched Cvt Change, Watched Store Change, Read of Uninitialized Store).
- **Suggest Deltas** (`deltaui.c`): Rasterize at sizes, DPI, Mono/Anti-Aliased, Proximity, sorted list of Glyph/Point/Size candidates.
- **Grid Fit Parameters** (`cvgridfit.c`): DPI, Pointsize X/Y, Scale X/Y the same, Mono/Anti-Aliased, Debug fpgm/prep.
- **Histograms** (`histograms.cpp`): HStem / VStem / BlueValues distribution with click-to-set Private-dict values.

---

## 12. Other dialogs

| Dialog | File | Key controls |
|---|---|---|
| **Find / Replace** | `searchview.c` | Search Pattern / Replace Pattern canvases, Find, Find Next, Find All, Replace, Replace All, Allow: Flipping / Scaling / Rotating, Match Fuzziness, Search Selected Chars Only, "Endpoints specify minimum length and direction only" |
| **Print / Display** | `displayfonts.c` | Full Font Display, Multi Size Glyphs, Full Page Glyphs, free-text sample (`sftextfield.c`) with per-run font, script&language, feature list, AA/bitmap, point/pixel size; Printer/File/Other command, Page Setup, Page Size, Copies, DPI, Pointsize |
| **Insert Text Outlines** | `displayfonts.c` | Bind to Path, Offset text from path by, Scale so text width matches path length, Rotate each glyph as a unit, Align At Start/Centered/At End |
| **Mass Glyph Rename** | `lookupui.c` | Rename all glyphs in the selection, By appending the suffix, To the glyph names starting at, Replace it / Append to it, "Optionally, add this mapping to the lookup subtable" |
| **Define Groups / Display By Groups** | `groupsdlg.c` | Group tree, Group Name, Glyphs, Identify by Name/Unicode, New Sub-Group, Set From Font, Select In Font, duplicate detection |
| **Auto Width** | `autowidth2dlg.c` | Separation, Min Bearing, Max Bearing, Height, Loop Count |
| **Set Width/LBearing/RBearing/VAdvance** | `fvmetricsdlg.c` | Set To / Increment By / Scale By modes |
| **Merge Fonts / Interpolate Fonts** | `fvfontsdlg.c` | Font list, Preserve cross-font kerning, interpolation Amount |
| **Generate Fonts** | `savefontdlg.c` | Output format list (TTF, OTF/CFF, CFF Bare, CFF CID, Type1/Type3/Type11, SVG, UFO, BDF, NFNT, dfont, WOFF/WOFF2), Apple/OpenType/Old-style 'kern'/Dummy DSIG/FFTM/Merge tables across fonts, Hints, Flex Hints, Hint Substitution, Round coordinates, Glyph names in TTF/OTF, Lookup Names, First 256 only, Force glyph names to namelist, Validate before saving, Append FONTLOG entry, BDF Resolution, Layer selector |
| **Open Font** | `openfontdlg.c` | File type filters (All Fonts / Outline / Bitmap / TrueType / OpenType / Type1 / PostScript / SVG / SFD / Backup SFD / Archives / Extract from PDF), Edit Font Filters, Force glyph names to namelist |
| **Plugin Configuration** | `pluginui.cpp` | Enable/Disable/Load/Configure/Delete, Up/Down/Top/Bottom order, PluginStartupMode (On/Off/Ask), Module & Package name, Open Web Page |
| **Execute Script** | `scriptingdlg.c` | FF native script vs Python radio, Call Script… |
| **Justification** | `justifydlg.c` | Justified Scripts / Languages, Extenders, Extend/Shrink Lookups On/Off, Extend/Shrink Max Lookups |
| **Horizontal/Vertical Baselines** | `basedlg.c` | Baseline tags romn, hang, ideo, idtp, math, icfb, icft; Default Baseline; per-script Min/Max extents; Set Feature Extents |
| **Align Points / Space Regions** | `alignment.c` | Horizontal/Vertical, max distance between points in a region, Make Parallel |
| **Copy/Compare Layers** | `layer2layer.c` | From/To layer, Clear destination layer before copy, Error Bound |
| **Strike Information (BDF)** | `bdfinfo.c` | BDF X-property table, Default This / Default All, New property |
| **Combinations** | `combinations.c` | Kern Pairs, Anchored Pairs (Anchor Control for Base/Mark), Ligatures browsers, Kern Size, Sort By |
| **Goto** | `gotodlg.c` | Glyph name/codepoint entry, Merge into selection |

---

## 13. Preferences (`prefs.c`, `visible_prefs_list`)

Top-level tabs, with nested sub-tabs marked ⤷:

1. **Generic** — theme file, OtherSubrs file, FreeType rasterizer in font view, FreeType antialias in outline view, splash screen, ExportClipboard, AutoSaveFrequency, RevisionsToRetain, UndoRedoLimitToSave, SaveEditorState, WarnScriptUnsaved, UsePlugins, DefaultShaper.
2. **New Font** — default encoding, Em-Square size, quadratic vs cubic splines, load-order preservation.
3. **Open Font** — cmap preference for CJK/Unicode dual-cmap fonts, ask which cmap, preserved table tags, initial displayed character, compact on open, UndoRedoLimitToLoad, OpenTypeLoadHintEqualityTolerance.
4. **Navigation** — GlyphAutoGoto, open new window on double-click, glyph count pushed to Metrics View.
5. **Editing** — ItalicConstrained, interpolate control points, snap distance, measure-tool snap, round to int on click, StopAtJoin, join limit, copy metadata, UndoDepth, flex-hint recompute, AutoKern dialog for new subtables, metrics-window shift increments.
   - ⤷ **Interface** — arrow-key motion amount, shift multiplier, highlight open paths, measure-tool H/V distances, handle size, inactive handle alpha, always show control points, preview on space bar, thick outline width.
   - ⤷ **Synchronize** — width change propagates to accented glyphs, LSB change adjusts references.
   - ⤷ **TT** — clear instructions when points renumber, copy TT instructions with glyphs.
   - ⤷ **Accents** — accent offset percent, center on full width vs lowest point, center on highest points vs middle, use spacing accents.
   - ⤷ **Apps** — autotrace vs potrace choice, autotrace args, ask each time, mf commands, retain background images, mf verbosity.
6. **Font Info** — default foundry, Vendor ID, default NameList, ignore Adobe PUA variants, allow full Unicode in glyph names, AddCharToNameList.
7. **Generate** — ask BDF resolutions, AutoHint changed glyphs before generating, PNG images in SFD, GenerateHintWidthEqualityTolerance, Apple/OpenType defaults, .g2n glyph-to-name map file, XUID base.
   - ⤷ **PS Hints** — max slope difference for H/V, serif-terminal slope tolerance, bounding-box hints, diagonal-stem-end hints, diagonal-intersection hints, generate diagonal stem hints.
   - ⤷ **TT Instrs** — instruct diagonal stems, instruct serifs, instruct ball terminals, interpolate strong points, counter control.
8. **OpenType** — Indic script tag version ('2' tags).

Additional preference surfaces: **Appearance Editor** (`MenuXRes`, GResource/theme colors, fonts, all `GResFont`/color resources such as `cvpalettebgcol`, metrics-view divider/kern/LSB colors), and **Hotkeys** (`gdraw/hotkeys.c`, user-editable `hotkeys` file, per-menu-item rebinding).

---

## 14. Default keyboard shortcuts (from `doc/sphinx/ui/misc/HotKeys.rst`)

Ctrl-A Select All · Ctrl-Shift-A Build Accented Glyph · Alt-Ctrl-A Select All Points · Ctrl-B Regenerate Bitmaps · Ctrl-Shift-B Bitmaps Available · Ctrl-C Copy · Ctrl-Shift-C Copy Fg to Bg · Alt-Ctrl-C Copy Lookup Data · Ctrl-D Show/Hide Points · Ctrl-Shift-D Correct Direction · Ctrl-E Find Problems · Ctrl-Shift-E Expand Stroke · Ctrl-F Fit in Window · Ctrl-Shift-F Font Info · Alt-Ctrl-F Find/Replace · Alt-Ctrl-Shift-F Replace With Reference · Ctrl-G Copy Reference · Ctrl-Shift-G Generate Fonts · Alt-Ctrl-G Generate Mac Family · Ctrl-H Open Outline · Ctrl-Shift-H AutoHint · Alt-Ctrl-H Review Hints · Ctrl-I Get Info · Ctrl-Shift-I Import · Alt-Ctrl-Shift-I Glyph Info · Alt-Ctrl-I Show Dependencies · Ctrl-J Open Bitmap · Ctrl-Shift-J Join · Ctrl-K Open Metrics · Ctrl-Shift-K Auto Kern · Alt-Ctrl-Shift-K Merge Feature Info · Ctrl-L Set LBearing · Ctrl-Shift-L Set Width · Ctrl-M Merge Point · Ctrl-Shift-M Simplify · Alt-Ctrl-Shift-M Simplify More · Ctrl-N New · Ctrl-O Open · Ctrl-Shift-O Remove Overlap · Ctrl-P Print · Ctrl-Shift-P Balance · Alt-Ctrl-P Display · Ctrl-Q Quit · Ctrl-Shift-Q Close · Ctrl-R Set RBearing · Ctrl-Shift-R Revert File · Ctrl-Alt-R Revert Glyph · Ctrl-S Save · Ctrl-Shift-S Save As · Ctrl-T AutoInstr · Ctrl-Shift-T AutoTrace · Ctrl-U Unlink Reference · Ctrl-V Paste · Ctrl-Shift-V Paste Into · Ctrl-W Copy Width · Ctrl-Shift-W Auto Width · Ctrl-X Cut · Ctrl-Shift-X Add Extrema · Ctrl-Y Redo · Ctrl-Shift-Y Add Points Of Inflection · Ctrl-Z Undo · Ctrl-Shift-Z Harmonize · Ctrl-\ Transform · Ctrl-Shift-_ Round to Int · Ctrl-1 Make First · Ctrl-2/3/4 point sizes & Curve/Corner/Tangent point · Ctrl-Shift-@ Average Points · Ctrl-Shift-# Space Points · Ctrl-5 Anti-Alias · Ctrl-6 Fit To Em · Ctrl-7/9 72/96 px · Ctrl-Shift-%/^/* cell grid sizes · Ctrl-0 Add Anchor Point · Ctrl-] / Ctrl-[ Next/Prev Glyph · Ctrl-Shift-} / { Next/Prev Point · Ctrl-. Execute Script / Select First Point · Ctrl-Shift-> Goto · Alt-Ctrl-. First Point Next Contour · Ctrl-, Select Point At · Ctrl-Shift-< Find In Font View · Alt-Ctrl-, Points on Selected Contours · Escape Deselect All · Ctrl-Escape Invert Selection · Backspace/Delete Clear · Ctrl-= Grid Fit Anti Alias · Ctrl-Shift-+ Bigger Pixel/Point Size · Alt-Ctrl-Shift-+ Zoom In · Ctrl-- Smaller Size · Alt-Ctrl-- Zoom Out · F1/Help Help. Alt-Ctrl-1..9 invoke user scripts.

Tool modifiers (outline view): Pointer Shift=constrain H/V/italic, Alt=prefer control points, Shift-Alt=constrain CP to original angle; Magnify Shift=square, Alt=minify; Freehand/Knife/point tools Shift=45° constrain; Ruler Shift=one direction, Alt=higher precision; Scale Shift=one axis or uniform; Rotate Shift=45° steps; Rect/Ellipse Shift=square/circle; Polygon/Star Shift=vertex on axis. Arrow keys: Ctrl/CapsLock=scroll, Alt=×10 step.

---

## 15. What FontForge is missing vs Glyphs / FontLab / FontCreator

Legend: **[MUST]** = required for a professional editor; **[HIGH]** = strong differentiator; **[NICE]** = polish.

**Design system / variable fonts**
1. **[MUST]** Variable-font design-space UI — axes, masters, instances, and a visual design-space map. FontForge has only Adobe MM (`mmdlg.c`, max 4 axes, PostScript-era) and no `fvar/gvar/HVAR/STAT` authoring at all.
2. **[MUST]** Brace layers / intermediate (support) layers per glyph for local interpolation control. Nothing equivalent; `Interpolate Fonts…` (`FVMenuInterpFonts`) is a one-shot font-wide blend.
3. **[MUST]** Live interpolation preview / slider scrubbing across the design space while drawing. FontForge requires generating a blended font.
4. **[MUST]** Automatic compatibility checking across masters (point count/order/start point/direction), with a "fix compatibility" workflow. FontForge only has `MM Validity Check`.
5. **[HIGH]** STAT table and named-instance/axis-value authoring UI. Absent.
6. **[HIGH]** Layers as first-class masters with per-master metrics, kerning and hinting. FontForge layers (`fontinfo.c` Layers tab, `cvpalettes.c` layer palette) are fore/back/guide-style, not masters.

**Components & reuse**
7. **[MUST]** Smart components / corner & cap components (Glyphs' smart glyphs, FontLab's Smart Corners). FontForge only has plain references (`CVCopyRef`, `FVMenuBuildAccent`).
8. **[HIGH]** Auto-alignment of components with anchors driving position, live and non-destructive. FontForge's `Build Accented Glyph` is a batch, one-time build.
9. **[NICE]** Component scaling/mirroring safety with automatic direction correction; FontForge surfaces "flipped reference" as a *problem* to fix manually (`problems.cpp`).

**OpenType features**
10. **[MUST]** Live feature preview with a real shaper in every editing surface, with feature checkboxes and script/language switching. FontForge has this only in the Metrics View and Display dialog; the Font View's `Display Substitutions…` is a crude substitute.
11. **[MUST]** A real `.fea` editor with syntax highlighting, autocomplete and inline compile errors. FontForge offers only import/export (`Save Feature File…`, `Import…` in the Lookups tab) plus an unfriendly lookup/subtable tree.
12. **[MUST]** Auto-generated feature code from glyph-name suffixes (`.sc`, `.ss01`, `.alt`) — Glyphs' automatic feature generation. FontForge requires hand-built lookups per feature (`lookupui.c`).
13. **[HIGH]** Named glyph classes reusable across features and kerning, edited in one place. FontForge keeps kern classes (`kernclass.c`), mark classes (`fontinfo.c`), and contextual classes (`contextchain.c`) in three unrelated editors.
14. **[HIGH]** Visual contextual-rule editor that shows rendered text, not the abstract Backtrack/Match/Lookahead grid of `contextchain.c`.

**Kerning & spacing**
15. **[MUST]** Kerning groups (left/right group per glyph, set from the glyph itself) rather than a global class matrix. `kernclass.c`'s two-list matrix is the single worst usability gap versus Glyphs/FontLab.
16. **[MUST]** Kern-in-context: type arbitrary text, kern with keyboard, exceptions created automatically from group pairs. FontForge's Metrics View kerns pairs but has no group-vs-exception model in the UI.
17. **[HIGH]** Metrics expressions/linked metrics (`=n`, `=|H`, `=H+10`) so sidebearings stay in sync. FontForge only has Auto Width (`autowidth2dlg.c`) and manual numbers.
18. **[HIGH]** Visual kerning-pair auditing (heat map / list of outliers / missing pairs).

**Color & modern formats**
19. **[MUST]** COLR/CPAL v0 + v1 colour-font authoring (palettes, gradients, composite modes). FontForge has no COLR support; its colour story is Type3 multi-layer (`cvstroke.c`) which no modern renderer uses.
20. **[HIGH]** SVG-in-OpenType (`SVG ` table) and CBDT/sbix bitmap-colour authoring.
21. **[HIGH]** WOFF2 and variable-font export as first-class targets in the export dialog (`savefontdlg.c` predates both).

**Editing UX**
22. **[MUST]** Modern non-modal, unlimited, coalesced undo with a visible history and font-wide undo. FontForge has per-glyph undo stacks with `UndoDepth`/`Remove Undoes` memory management (`edlist`, `sfundo.c`) — an anachronism.
23. **[MUST]** Tabbed/document-based window management instead of one OS window per font/glyph/metrics view plus floating palettes (`fontview.c`, `charview.c`, `metricsview.c`, `cvpalettes.c`). Glyph tabs exist (`CVMenuShowTabs`) but the model is still window-per-thing.
24. **[MUST]** Non-destructive live filters/transformations (Glyphs' filters, FontLab's actions) applied at export, not baked into outlines. Every FontForge style/effect (`scstylesui.c`, `effectsui.c`, `cvstroke.c`) is destructive.
25. **[HIGH]** Live corner/round-corner/pen-shape effects that stay editable.
26. **[HIGH]** True pen tool ergonomics: rubber-band bezier preview, on-canvas handle constraints, alt-drag to break tangency, path direction arrows by default. FontForge's `cvaddpoints.c`/`cvpointer.c` are functional but dated.
27. **[HIGH]** Snapping and measurement guides that are persistent, named, and global/per-glyph, with an on-canvas measurement HUD. FontForge has a Guide layer and a ruler tool but no smart guides/alignment feedback.
28. **[NICE]** Background image/tracing workflow with per-image transform handles rather than the import-then-autotrace pipeline (`cvimportdlg.c`, `FVMenuAutotrace`).

**Glyph management & browsing**
29. **[MUST]** Glyph filters, saved smart searches, tags/labels, and sidebar categories (Letters/Numbers/Punctuation/Script/Language). FontForge offers only `Define Groups…` (`groupsdlg.c`), `Select by Wildcard/Script/Color`, and encoding-order browsing.
30. **[MUST]** Glyph-set templates and "add glyphs from recipe" (`A-Z a-z /one.tf=/one`) driven by a language/charset database. FontForge has `Create Named Glyphs…` + namelists only.
31. **[HIGH]** A modern text-preview/proofing panel docked in the app with waterfall, paragraph and word-list modes. FontForge splits this across the Metrics View, `displayfonts.c` (Print/Display) and word lists (`wordlistparser.c`).
32. **[NICE]** Per-glyph annotations/status colours integrated into a production workflow (FontForge has cell colours and comments but no status pipeline).

**Interoperability & engineering**
33. **[MUST]** First-class UFO3 + designspace as the native format (with round-trip fidelity), not SFD. FontForge treats UFO as an import/export format.
34. **[MUST]** Git-friendly project files and diffing. SFD is a single opaque blob; `Compare Fonts…` (`showatt.c`) is an in-app diff, not a VCS story.
35. **[HIGH]** Integrated modern toolchain: fontmake/fontTools/HarfBuzz/ots as libraries, plus autohinting via ttfautohint/psautohint. FontForge ships its own aging autohinter and TT instructor.
36. **[HIGH]** Batch/CLI build pipeline with a project-level "export all instances" step. FontForge scripting (`scriptingdlg.c`, `pythonui.c`) exists but there is no build-recipe UI.
37. **[NICE]** Plugin API with modern packaging and a marketplace; `pluginui.cpp` is a start but limited to Python entry points.

**Presentation & accessibility**
38. **[MUST]** HiDPI-correct, GPU-accelerated, themable rendering. FontForge draws with its own `gdraw` toolkit; `MenuXRes` theming is a resource-file editor.
39. **[HIGH]** Discoverable command palette / searchable menu (FontForge has ~450 menu commands spread over 11 menus with no search).
40. **[NICE]** Contextual inspector panel replacing the ~40 separate modal dialogs enumerated above — the single biggest structural difference from Glyphs/FontLab.
