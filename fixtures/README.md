# Font fixtures

All files in `fixtures/fonts/` are freely licensed and smaller than 2 MB. Hashes
are SHA-256 values for the committed bytes.

| File | Size (bytes) | Source | License | SHA-256 |
| --- | ---: | --- | --- | --- |
| `DejaVuSans.ttf` | 759720 | Ubuntu `fonts-dejavu-core` 2.37-8, `/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf`; [DejaVu Fonts](https://dejavu-fonts.github.io/) | [DejaVu license](https://dejavu-fonts.github.io/License.html) | `ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280` |
| `DejaVuSans.woff` | 379400 | Wrapped from the committed `DejaVuSans.ttf` with fontTools 4.63.0 | DejaVu license | `2ed2811b1f4d1d04c4def150516901b35c3025b4a29dfdfb32c90a5e588ce7af` |
| `NotoSans-Regular.ttf` | 569208 | [notofonts/noto-fonts static hinted TTF](https://github.com/notofonts/noto-fonts/blob/ffebf8c1ee449e544955a7e813c54f9b73848eac/hinted/ttf/NotoSans/NotoSans-Regular.ttf) | [SIL Open Font License 1.1](https://scripts.sil.org/OFL) | `b85c38ecea8a7cfb39c24e395a4007474fa5a4fc864f6ee33309eb4948d232d5` |
| `test1012.otf` | 1696 | [FontForge test fixture](https://github.com/fontforge/fontforge/blob/dc05099ff6b488d459c8a59259807007dd59d9a5/tests/fonts/test1012.otf) | GPL-3.0-or-later (FontForge test data) | `efc3fb3196e4aa87df6bebfe75c146737a1401eff30aabfe5eeb4028ebae799f` |
| `CantarellMin.woff2` | 2300 | [FontForge test fixture](https://github.com/fontforge/fontforge/blob/dc05099ff6b488d459c8a59259807007dd59d9a5/tests/fonts/CantarellMin.woff2) | SIL Open Font License 1.1 (Cantarell) | `af7b646662ea3f3aa31e45d9782d087caf9e3e31ac5636cb6ac146b5fa5010ba` |
| `NotoSerifTibetan-Regular.ttf` | 632900 | [FontForge test fixture](https://github.com/fontforge/fontforge/blob/dc05099ff6b488d459c8a59259807007dd59d9a5/tests/fonts/NotoSerifTibetan-Regular.ttf) | SIL Open Font License 1.1 (Noto) | `3b36e25ff1e0529a9e7d764bf98ff29371fdb08dba04bc6e1f682d93974286a7` |

## Optional external fixtures

These fonts are intentionally not committed. `fw_fixtures::external` returns
their paths only when they exist on the current machine, so dependent tests can
skip cleanly.

| Key | Expected path | Purpose |
| --- | --- | --- |
| `source-han-sans-sc-otf` | `/home/chen/githubprojects/pixelfonts/SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf` | Large Source Han Sans SC static OTF performance and coverage tests |
| `fontforge-tests` | `<workspace>/fontforge/tests/fonts` | Broader compatibility corpus from the local FontForge checkout |
