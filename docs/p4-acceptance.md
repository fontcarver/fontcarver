# P4 variable fonts and CJK acceptance

Status: implemented and verified on 2026-09-22 within the explicit boundaries
below. The P4 export/install exit criterion passed on Linux. This report does not
claim unrestricted source-format coverage or macOS/Windows/native-input acceptance.

## Implemented paths

| Requirement | Editing and persistence | Direct evidence |
| --- | --- | --- |
| Axes, masters, instances | Atomic revision-checked editor; mapped coordinates; cloning/removal; source-reference migration on axis rename; FCF | Model/document/API tests, draft failure/retry tests, native save/reopen |
| Interpolation and repair | Sparse multi-axis model and intermediate peaks; live Font Info/glyph-inspector preview; per-group contour/start/order repair | Endpoint/interior tests; geometry/undo tests; real CJK browser preview |
| Variable exports | TTF `gvar`, CFF2 OTF, variable WOFF/WOFF2, named/static instances and source subsets | Compiler tests plus independent FontTools and FreeType sampling |
| Brace/bracket sources | Layer authoring, conditional static outlines and variable `rvrn`; component-dependent alternates | Boundary shaping tests; native Glyphs/designspace round trips; external-rule edits |
| Smart components | Named local axes and rectangular source grids; per-use parameters; nested resolution and decomposition | Model/geometry/compiler and API edit/decompose/undo/save tests; browser editing |
| Vertical text and metrics | Layer overrides, master defaults, `vert`/`vrt2` controls, model/manual `vkrn`, group exceptions | TTF/CFF2 compilation, mapped kerning, script registration, text layout tests and browser workflow |
| CJK metadata | Unicode 17.0 Unihan, normalized reading queries, combined encoding filters, charset recipes, sorting | All 102,998 records indexed; same-encoding conjunction tests; browser radical/reading metadata |
| Batch spacing | Captured selection, explicit preview, optimistic generation check, one/all masters, dependency-aware centering | Both component selection orders, stale/invalid requests and atomic undo tests; browser save verification |
| CID | Explicit ROS/map editor, Identity fill, numeric order, rename/delete maintenance, CID-keyed CFF | Command/API/native persistence tests; independent FontTools ROS/FDArray/FDSelect/charset decoding |
| Reverse cmap | Incremental document Unicode index, indexed text widths | Rename/re-encode/delete/history tests and 65k timed integration test |

New UI strings have matching English and Simplified Chinese entries. Source code
comments, schema updates and developer/user documentation are English. See
[user instructions](user/variable-and-cjk.md).

## Real-outline export and installation

`scripts/p4_variable_acceptance.py` constructs two compatible source masters from
locally installed Noto Sans Regular/Bold and the external Source Han Sans SC
Regular/Bold corpus. It retains upstream license metadata and gives the derived
fixtures distinct `FontCarverP4Latin` and `FontCarverP4CJK` names. The selection
contains 53 distinct Latin and 256 distinct CJK glyphs with compatible original
outlines. It is a real-outline subset acceptance, not a full CJK variable export
benchmark. Original input files are not changed or redistributed.

All four exports have a 400–700 weight axis and a Medium named instance at 550.
FontTools independently checks each glyph's outline bounds and advance against
the source interpolation at 400, 475, 550, 625 and 700:

| Output | Greatest bounds deviation in font units |
| --- | ---: |
| Latin variable TTF | 0 |
| Latin CFF2 OTF | 0.000046 |
| CJK variable TTF | 0.548454 |
| CJK CFF2 OTF | 0 |

The TrueType CJK deviation is within the cubic-to-quadratic approximation budget.
Both static Medium exports have no `fvar` and have weight class 550. Variable
outputs contain HVAR/VVAR and vhea/vmtx; CFF2 also has VORG.

`scripts/p4_install_acceptance.py` installed all four faces in a uniquely created
user font directory on Linux, refreshed Fontconfig, found every installed file,
and resolved both family names with `fc-match`. FreeType rendered 16 encoded
characters at two pixel sizes and five axis positions per face: **640 samples**.
FreeType and HarfBuzz independently verified variable vertical advances at all
five positions. The temporary installation directory was then removed and the
font cache refreshed. This is an actual temporary Linux installation check.

Reports, hashes, source masters and outputs are in
`/tmp/fontcarver-p4-real-acceptance/`; `report.json` records input provenance and
numerical comparisons, and `installation.json` records installation/rendering.

Reproduction:

```sh
cargo build -p fc-cli
python3 scripts/p4_variable_acceptance.py --output /tmp/fontcarver-p4-real-acceptance \
  --cjk-regular /path/to/SourceHanSansSC-Regular.otf \
  --cjk-bold /path/to/SourceHanSansSC-Bold.otf
python3 scripts/p4_install_acceptance.py /tmp/fontcarver-p4-real-acceptance
```

The first script requires fontTools; the second additionally requires
freetype-py, uharfbuzz and Linux Fontconfig. These are external verification
utilities and are not runtime dependencies of FontCarver.

## Regression and browser evidence

- Rust workspace regression: **993 passed, 2 ignored**, excluding the known
  `licensed_real_font_inventory_reports_actual_coverage_without_mutation`
  Tibetan fixture failure independently reproduced before P4. Later focused
  suites pass the two newly added source-repair/conditional-dependency tests:
  **995 distinct passing tests** in total. No unrestricted green workspace is
  claimed. The two ignored release-only tests remain unchanged.
- Rust formatting and workspace Clippy, all targets with warnings denied, pass.
- Frontend regression: **1,114 tests passed across 136 files**. TypeScript,
  ESLint, Prettier and the production build pass. The existing large-chunk
  build warning remains; it is not a failed build.
- The final embedded Linux desktop build passes with the current production
  assets (`cargo build -p fontcarver --features tauri/custom-protocol`).
- P3 bitmap browser regression: **4 passed**, including pixel edits, rollback,
  save/export/reopen, tracing, regeneration and strike tools.
- Browser session `fontcarver-p4-20260922` verified axis/master/instance editing,
  variable TTF/CFF2 exports, local component parameters, vertical spacing, Unihan
  filters/metadata, CID mapping and batch spacing persisted in FCF.
- Actual CJK browser evidence:
  `/tmp/fontcarver-p4-real-cjk-preview.png` and
  `/tmp/fontcarver-p4-editor-interpolation.png`. The latter displays the current
  glyph's Medium interpolation beside its editable Regular source and reports
  256 glyphs. The browser session was closed after verification.
- Earlier focused screenshots are `/tmp/fontcarver-p4-unihan-filter.png`,
  `/tmp/fontcarver-p4-unihan-metadata.png`, `/tmp/fontcarver-p4-cjk-batch.png`,
  `/tmp/fontcarver-p4-cid-editor.png` and `/tmp/fontcarver-p4-vertical.png`.

The release 65,535-glyph CJK filter test measured **51.7 ms** for combined
radical/stroke/reading search and **89.0 ms** for batch preview. The indexed
65,536-glyph text-width test performs 1,000 lookups within one second. These
performance tests use constructed glyphs, separately from real-outline exports.

## Explicit boundaries

- Bracket authoring currently represents one axis per region. Overlaps, including
  conflicting alternates inherited through components, fail instead of silently
  selecting one. A variable alternate requires a default-master source.
- Native designspace `rvrn` single-axis rules convert to editable layers. Compound
  conditions, chained substitutions, late/custom-feature processing and alternate
  supplemental sources remain lossless opaque source rules; interpolation and
  binary compilation reject these until explicitly converted. Source export
  preserves them. Glyphs compound bracket rules are rejected explicitly.
- Smart grids require their neighboring corners, at most eight local axes and
  compatible source topology. Glyphs smart-pole export requires endpoint sources;
  FCF/UFO can retain interior local sources. Background/backup drawings are not
  evaluated as compilation sources.
- CFF2 uses a single FontDICT without hint programs. It checks finite operands,
  normalized/quantized region validity, source compatibility, 505 active regions
  per glyph, 65,535-byte charstrings and aggregate resource limits. It reports
  representability failures rather than writing a fallback static font.
- CID CFF export requires complete, unique mappings, with `.notdef` at CID 0.
  The charset filters are repertoire coverage tools, not encoding certification.
- UI interpolation is an adjacent read-only view. Contour-topology changes,
  richer STAT authoring, instance-to-master creation and the broader professional
  feature checklist remain part of the subsequent professional-finish audit.
- An embedded Linux desktop build is verified without launching it. Native input
  automation remains unauthorized under the earlier user instruction recorded in
  `docs/p1-acceptance.md`; macOS/Windows installation and desktop interaction are
  unverified. P5 release packaging is a separate task.

Specification references: [designspace rules](https://fonttools.readthedocs.io/en/latest/designspaceLib/xml.html#rules-element),
[OpenType CFF2](https://learn.microsoft.com/en-us/typography/opentype/spec/cff2).
Dataset provenance and pinned hashes are in `crates/fc-glyphdata/data/README.md`
and `THIRD_PARTY.md`. No FontForge executable or source code was used.
