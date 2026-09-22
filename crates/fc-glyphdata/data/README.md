# Vendored glyph data

These text files are build inputs. `build.rs` converts them into sorted Rust
tables; the original files stay here so updates are reviewable and reproducible.

| File | Source | Licence / terms | SHA-256 |
| --- | --- | --- | --- |
| `aglfn.txt` | <https://raw.githubusercontent.com/adobe-type-tools/agl-aglfn/master/aglfn.txt> | BSD-3-Clause; the complete notice is retained in the file and at <https://github.com/adobe-type-tools/agl-aglfn/blob/master/LICENSE.md>. | `ed735a9ea58549b4cb2b8e804341548adb25aff43e81001ac7f3917247cacca1` |
| `Blocks.txt` | <https://www.unicode.org/Public/17.0.0/ucd/Blocks.txt> | Unicode License v3; see <https://www.unicode.org/license.txt>. | `c0edefaf1a19771e830a82735472716af6bf3c3975f6c2a23ffbe2580fbbcb15` |
| `GB2312.TXT` | Historical Unicode mapping `EASTASIA/GB/GB2312.TXT`, preserved in <https://www.haible.de/bruno/charsets/conversion-tables/GB2312.tar.bz2> | Unicode permission notice retained in the file. The original Unicode URL is no longer published. | `a340f97d05efa88d27163165dcd0d63cedb4edc0eac714d782fa31dcfc8da732` |
| `BIG5.TXT` | <https://www.unicode.org/Public/MAPPINGS/OBSOLETE/EASTASIA/OTHER/BIG5.TXT> | Unicode terms of use and licence referenced in the retained file header. | `d1b60c58a1d327918f1616a620162c60ad2079a229ee42a73488f186e11f3aac` |

`Blocks.txt` is Unicode 17.0.0. The legacy mapping files intentionally retain
their historical Unicode versions because they define legacy charset
repertoires rather than current Unicode properties.

## P4 Unihan and CJK repertoires

The Unihan index pins Unicode **17.0.0** and preserves the original values of
`kRSUnicode`, `kTotalStrokes`, `kMandarin`, `kCantonese`, `kJapaneseOn`,
`kJapaneseKun`, `kKorean`, and `kIICore`. The three plane shards contain 102,998
records; each is below the repository's 2 MB file limit. Readers retain all
radical and stroke alternatives; search normalizes reading tones while display
uses the original source spelling. The index is generated at build time, requires
no network at runtime, and uses binary search without expanding the whole Unicode
space on each request.

| Input | Pinned source | SHA-256 |
| --- | --- | --- |
| Unihan archive | <https://www.unicode.org/Public/17.0.0/ucd/Unihan.zip> | `f7a48b2b545acfaa77b2d607ae28747404ce02baefee16396c5d2d7a8ef34b5e` |
| JIS X 0208 | <https://www.unicode.org/Public/MAPPINGS/OBSOLETE/EASTASIA/JIS/JIS0208.TXT> | `1c571870457f19c97720631fa83ee491549a96ba1436da1296786a67d8632e87` |
| KS X 1001 | <https://www.unicode.org/Public/MAPPINGS/OBSOLETE/EASTASIA/KSC/KSX1001.TXT> | `d8d2a35206ac0ea2865f5d801c9d6717f735bf46f263a658a64a960abe59e371` |
| GB 18030 mapping | <https://raw.githubusercontent.com/unicode-org/icu/release-77-1/icu4c/source/data/mappings/gb18030-2022.ucm> | `50b16dfde5438f4c63ff59adcf3a5a6d8d3e6987f2b7c31e4383aaa5a6f6b6e8` |

Unihan and ICU data use Unicode License v3; the complete notice is in
`UNICODE-LICENSE.txt`. The historical JIS/KS files retain their own Unicode
permission notices. ICU's additional IBM copyright is retained in the generated
GB repertoire header. No FontForge data or code is used.

Reproduce the derived files after downloading the two pinned inputs above:

```sh
python3 scripts/extract_unihan.py --unihan /path/to/Unihan.zip \
  --gb18030 /path/to/gb18030-2022.ucm
```

The script verifies source hashes before writing. `build_unihan.rs` validates
scalar values, column counts and duplicates. JIS uses the third mapping column
(the first two contain Shift-JIS and JIS encodings); KS uses the second column.
The resulting unique repertoires contain 6,879 JIS and 8,224 KS scalars.

The GB 18030-2022 Level 1 **assigned Unicode font repertoire** contains 28,606
scalars: assigned round-trip single/double-byte ICU mappings, Extension A through
U+4DB5 and unified ideographs through U+9FEF. Private-use mappings without assigned
characters and the nine compatibility characters removed by Annex D.3 are
excluded. This character coverage filter is not an encoding implementation or a
claim of GB certification. The implementation requirements and 2022 changes are
summarized in [Unicode L2/22-274](https://www.unicode.org/L2/L2022/22274-disruptive-changes.pdf).
IICore uses the pinned Unihan property and contains 9,810 scalars.
