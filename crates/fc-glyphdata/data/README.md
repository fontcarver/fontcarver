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
