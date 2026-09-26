# Font fixtures

All files in `fixtures/fonts/` are freely licensed and smaller than 2 MB. Hashes
are SHA-256 values for the committed bytes.

| File                           | Size (bytes) | Source                                                                                                                                                                   | License                                                                                         | SHA-256                                                            |
| ------------------------------ | -----------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| `DejaVuSans.ttf`               |       759720 | Ubuntu `fonts-dejavu-core` 2.37-8, `/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf`; [DejaVu Fonts](https://dejavu-fonts.github.io/)                                    | [DejaVu license](https://dejavu-fonts.github.io/License.html)                                   | `ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280` |
| `DejaVuSans.woff`              |       379400 | Wrapped from the committed `DejaVuSans.ttf` with fontTools 4.63.0                                                                                                        | DejaVu license                                                                                  | `2ed2811b1f4d1d04c4def150516901b35c3025b4a29dfdfb32c90a5e588ce7af` |
| `DejaVuSansMin.woff2`          |        11332 | Subset from the committed DejaVu Sans 2.37 TTF with fontTools 4.63.0; build command below                                                                                | DejaVu license (Bitstream Vera license plus public-domain DejaVu changes)                       | `5a8a0335559e7152e36c018b86b8e518d65a0cfce1a5be58fc684525ac8ad821` |
| `NotoSans-Regular.ttf`         |       569208 | [notofonts/noto-fonts static hinted TTF](https://github.com/notofonts/noto-fonts/blob/ffebf8c1ee449e544955a7e813c54f9b73848eac/hinted/ttf/NotoSans/NotoSans-Regular.ttf) | [SIL Open Font License 1.1](https://scripts.sil.org/OFL)                                        | `b85c38ecea8a7cfb39c24e395a4007474fa5a4fc864f6ee33309eb4948d232d5` |
| `NotoSerifTibetan-Regular.ttf` |       632900 | [FontForge test fixture](https://github.com/fontforge/fontforge/blob/dc05099ff6b488d459c8a59259807007dd59d9a5/tests/fonts/NotoSerifTibetan-Regular.ttf)                  | SIL Open Font License 1.1 (Noto)                                                                | `3b36e25ff1e0529a9e7d764bf98ff29371fdb08dba04bc6e1f682d93974286a7` |
| `SourceSans3Min.otf`           |         3480 | [Adobe Source Sans 3 Regular 3.052, tag `3.052R`](https://raw.githubusercontent.com/adobe-fonts/source-sans/3.052R/OTF/SourceSans3-Regular.otf); build command below     | [SIL Open Font License 1.1](https://github.com/adobe-fonts/source-sans/blob/3.052R/LICENSE.md)  | `556ca78a1ff146426d00a09c647e27900ffd7511d88ea4f71f591cc2b862ec42` |
| `SourceSerif4Min.otf`          |         3112 | [Adobe Source Serif 4 Regular 4.005, tag `4.005R`](https://raw.githubusercontent.com/adobe-fonts/source-serif/4.005R/OTF/SourceSerif4-Regular.otf); build command below  | [SIL Open Font License 1.1](https://github.com/adobe-fonts/source-serif/blob/4.005R/LICENSE.md) | `6effaced2cc50d8b6fb0fa818f5c192e0408a9facc391594934b002de257031e` |

## Generated subsets

With fontTools 4.63.0 and its Brotli support installed, rebuild all three small
fixtures from their pinned sources with:

```sh
python3 fixtures/build_font_subsets.py
```

The script runs the equivalent of these exact `pyftsubset` selections:

```sh
common_options=(
  '--layout-features=*' --no-layout-closure --glyph-names '--name-IDs=*'
  --name-legacy '--name-languages=*' --symbol-cmap --legacy-cmap
  --notdef-glyph --notdef-outline --no-recommended-glyphs
  --no-recalc-timestamp
)

pyftsubset fixtures/fonts/DejaVuSans.ttf \
  --output-file=fixtures/fonts/DejaVuSansMin.woff2 --flavor=woff2 \
  --unicodes=U+0041-005A,U+0061-007A --hinting "${common_options[@]}"

pyftsubset /tmp/SourceSans3-Regular.otf \
  --output-file=fixtures/fonts/SourceSans3Min.otf \
  --unicodes=U+0020,U+0041,U+0056,U+006F --hinting "${common_options[@]}"

pyftsubset /tmp/SourceSerif4-Regular.otf \
  --output-file=fixtures/fonts/SourceSerif4Min.otf \
  --glyphs=.notdef,o --no-hinting "${common_options[@]}"
```

`build_font_subsets.py` downloads the two `/tmp` inputs from the exact URLs in
the table, then changes the modified Adobe fonts' primary `name` and CFF names
to `FontWorkshop CFF Sans Fixture` and `FontWorkshop CFF Serif Fixture`. This
honors the upstream Reserved Font Name `Source`; the upstream statements and
full OFL 1.1 text accompany the fonts in `LICENSE-OFL.txt`.

The hinted Source Sans subset contains `.notdef`, `space`, `A`, `V`, and `o`; it
retains Type 2 hints, StandardEncoding, and the CFF Private DICT. The Source
Serif subset contains `.notdef` and `o`; `--no-hinting` deliberately reproduces
the old unhinted-CFF fixture's no-warning test case while retaining
StandardEncoding and non-hint Private DICT values. The DejaVu WOFF2 contains
`.notdef` plus ASCII `A`–`Z` and `a`–`z` (53 glyphs).

## Optional external fixtures

These fonts are intentionally not committed. `fc_fixtures::external` returns
their paths only when they exist on the current machine, so dependent tests can
skip cleanly. Source Han Sans uses `FC_SOURCE_HAN_SANS` when set; otherwise it
is resolved from the sibling `SourceHanSanOTF` checkout beside the primary
FontCarver repository, including when a test runs from a linked worktree.

| Key                      | Expected path                                                                                                  | Purpose                                                            |
| ------------------------ | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| `source-han-sans-sc-otf` | `$FC_SOURCE_HAN_SANS` or `<primary-repo-parent>/SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf` | Large Source Han Sans SC static OTF performance and coverage tests |
| `fontforge-tests`        | `<workspace>/fontforge/tests/fonts`                                                                            | Broader compatibility corpus from the local FontForge checkout     |

The standalone `scripts/p5_math_acceptance.py` also accepts external paths.
The MATH acceptance corpus includes
`/usr/share/fonts/truetype/dejavu/DejaVuMathTeXGyre.ttf` (DejaVu 2.37,
577,192 bytes; SHA-256
`75a6cb527b0175f9d13af29c33a6915ae38645016c6fe9881653155cd1f62485`).
It is read from the system installation and is not redistributed. Its 4,282
glyphs exercise a substantially larger MATH table than the committed DejaVu
Sans fixture. See [MATH acceptance](../docs/p5-math-acceptance.md).
