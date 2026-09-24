# Fixture relicensing report

Date: 2026-09-04  
Branch: `p1-outline-editing`
Commit: `3acdc1c327302773494fa2301f734427732782f6`

## Outcome

The three redistributed GPL/AFPL fixtures were removed and replaced by one
Bitstream Vera/DejaVu WOFF2 subset and two SIL OFL 1.1 CFF OpenType subsets.
All seven committed files in `fixtures/fonts/` are now licensed under SIL OFL
1.1 or the permissive Bitstream Vera/DejaVu terms. No source file was adapted
from FontForge.

The two Adobe-derived subsets are OFL Modified Versions. Their primary `name`
records and CFF FontName/FamilyName/FullName values were changed to avoid the
Reserved Font Name `Source`. The original copyright, name ID 13 OFL notice, and
name ID 14 URL remain embedded. `fixtures/LICENSE-OFL.txt` contains the full OFL
1.1 text and the separate upstream copyright/RFN statements for each Adobe
font.

## Fixture changes

### `DejaVuSansMin.woff2`

- Replaces: `CantarellMin.woff2` (2009 GPLv3-or-later Cantarell).
- Upstream: the committed `fixtures/fonts/DejaVuSans.ttf`, DejaVu Sans 2.37,
  SHA-256 `ae7b7855e115a5966d8b1b3f80f254ccc117ec86f9965e202ee2940453837280`.
- License evidence: output name IDs 0/13/14 are retained from the source. ID 13
  contains the Bitstream Vera Fonts license, the public-domain DejaVu changes,
  and the Arev notice; ID 14 is
  `http://dejavu.sourceforge.net/wiki/index.php/License`.
- Subset command: `python3 fixtures/build_font_subsets.py`, whose WOFF2
  selection is `pyftsubset fixtures/fonts/DejaVuSans.ttf
  --output-file=fixtures/fonts/DejaVuSansMin.woff2 --flavor=woff2
  --unicodes=U+0041-005A,U+0061-007A --hinting` plus the common preservation
  options printed in `fixtures/README.md`.
- Contents: `.notdef`, `A`-`Z`, and `a`-`z`; 53 glyphs; TrueType `glyf`
  outlines in a WOFF2 container.
- Size/hash: 11,332 bytes;
  `5a8a0335559e7152e36c018b86b8e518d65a0cfce1a5be58fc684525ac8ad821`.

### `SourceSans3Min.otf`

- Replaces: `test1012.otf` (FontForge GPL-3.0-or-later test data).
- Exact upstream:
  `https://raw.githubusercontent.com/adobe-fonts/source-sans/3.052R/OTF/SourceSans3-Regular.otf`,
  tag `3.052R`, name-table version 3.052. The downloaded source was 334,924
  bytes with SHA-256
  `08df266400933d3178d081a45f94a08814c3e55b4b7dd2e0ff69cb1329f13ab6`.
- License evidence: the tag's
  `https://github.com/adobe-fonts/source-sans/blob/3.052R/LICENSE.md` is SIL OFL
  1.1 and states `Copyright 2010-2022 Adobe ... with Reserved Font Name
  'Source'`. The built font's ID 0 also states Adobe and RFN `Source`; ID 13
  identifies OFL 1.1; ID 14 is `http://scripts.sil.org/OFL`.
- Subset command: `python3 fixtures/build_font_subsets.py`, whose selection is
  `pyftsubset /tmp/SourceSans3-Regular.otf
  --output-file=fixtures/fonts/SourceSans3Min.otf
  --unicodes=U+0020,U+0041,U+0056,U+006F --hinting` plus the documented common
  preservation options. The same script changes the primary family to
  `FontWorkshop CFF Sans Fixture` and PostScript name to
  `FontWorkshopCFFSansFixture-Regular` in both `name` and CFF.
- Contents: `.notdef`, `space`, `A`, `V`, and `o`; 5 glyphs; `OTTO` with `CFF `
  and no `glyf`; StandardEncoding. It retains Type 2 hint operators and Private
  DICT data including BlueValues, StdHW 73, StdVW 84, StemSnapH 73, StemSnapV
  84, and nominalWidthX 518.
- Size/hash: 3,480 bytes;
  `556ca78a1ff146426d00a09c647e27900ffd7511d88ea4f71f591cc2b862ec42`.

### `SourceSerif4Min.otf`

- Replaces: `NimbusRoman-Regular-o.otf` (AFPL/GPL URW Base35 data).
- Exact upstream:
  `https://raw.githubusercontent.com/adobe-fonts/source-serif/4.005R/OTF/SourceSerif4-Regular.otf`,
  tag `4.005R`, name-table version 4.005. The downloaded source was 241,392
  bytes with SHA-256
  `edf160d0d584deee8a3bb2c3371b2a7624ca63580fbe02c57c1f4c91e84d8787`.
- License evidence: the tag's
  `https://github.com/adobe-fonts/source-serif/blob/4.005R/LICENSE.md` is SIL OFL
  1.1 and states `Copyright 2014 - 2023 Adobe ... with Reserved Font Name
  'Source'`. The output's name IDs 0, 13, and 14 provide the same RFN and OFL
  evidence.
- Subset command: `python3 fixtures/build_font_subsets.py`, whose selection is
  `pyftsubset /tmp/SourceSerif4-Regular.otf
  --output-file=fixtures/fonts/SourceSerif4Min.otf --glyphs=.notdef,o
  --no-hinting` plus the documented common preservation options. The script
  changes the primary family to `FontWorkshop CFF Serif Fixture` and
  PostScript name to `FontWorkshopCFFSerifFixture-Regular` in `name` and CFF.
- Contents: `.notdef` and `o`; 2 glyphs; `OTTO` with `CFF ` and no `glyf`;
  StandardEncoding. `--no-hinting` intentionally recreates Nimbus's distinct
  unhinted-CFF/no-warning property, while retaining its meaningful Private DICT
  value `nominalWidthX = 603`.
- Size/hash: 3,112 bytes;
  `6effaced2cc50d8b6fb0fa818f5c192e0408a9facc391594934b002de257031e`.

The builder was run twice and all three SHA-256 values were byte-identical.
fontTools 4.63.0 and Brotli were used. The script validates the container and
outline type, glyph counts, family names, OFL/DejaVu name records, CFF
StandardEncoding, hint/private-dictionary properties, and the 300 KB CFF limit.

## Test and documentation changes

- `crates/fw-sfnt/tests/read_containers.rs`: the WOFF2 path now names
  `DejaVuSansMin.woff2`; the weak nonempty assertion is made exact at 53 glyphs
  and the decoded family is checked as `DejaVu Sans`. Source provenance remains
  checked as `woff2`.
- `crates/fw-sfnt/tests/read_otf.rs`: the hinted CFF case uses
  `SourceSans3Min.otf`. The old source-specific `uni0627` assertion became the
  equivalent CFF-charset/cmap assertion that U+0041 resolves to glyph `A`, since
  Source Sans 3 has no Arabic repertoire. Tests now pin the measured family,
  style, UPEM, glyph count, nominal width, BlueValues, standard stems and stem
  snaps, cubic-only contours, and `cff-hints-not-imported` warning.
- `crates/fw-sfnt/tests/read_otf.rs`: the committed round-`o` case uses the
  deliberately unhinted `SourceSerif4Min.otf`, checks its two glyphs and Private
  DICT nominal width, and preserves the no-CFF-hint-warning assertion. The old
  environment-dependent system Nimbus test now checks the hinted Source Sans
  `o` deterministically, retaining a second curve-bearing CFF check without an
  AFPL/GPL font reference. The all-fixtures contour corpus lists all new files.
- `crates/fw-sfnt/tests/robustness.rs`: all three replacements participate in
  every 10%-step truncation and panic-safety check, so each malformed-input case
  is still derived from a valid fixture.
- `crates/fw-api/tests/dispatch.rs`: the CFF warning propagation test now opens
  `SourceSans3Min.otf` and retains the same warning assertion.
- `crates/fw-fixtures/src/lib.rs`: inspected; no filename was embedded in the
  helper, so no code change was required.
- `fixtures/README.md` and `THIRD_PARTY.md`: removed the three disallowed rows,
  documented the three replacements, and state that all committed font
  fixtures are now OFL or Vera/DejaVu licensed.
- `docs/p0-acceptance.md`: refreshed the fixture inspection matrix with the
  measured families, glyph counts, formats, and warnings.
- Historical FontForge research, plan, ledger, and prior licensing-report text
  still names the removed inputs as historical records; no active source, test,
  fixture manifest, or current third-party table refers to them.

`test1012.otf` also happened to contain an Arabic right-to-left GPOS lookup, but
no FontWorkshop test asserted that lookup's behavior; the CFF test exercised its
charset naming, cmap, cubic charstrings, Private DICT, and hint warning. The new
Source Sans fixture retains subsetted GPOS/GSUB/GDEF/BASE tables, while the
asserted CFF coverage is preserved and made more explicit.

## Verification

- `python3 fixtures/build_font_subsets.py` — passed twice; deterministic hashes,
  correct WOFF2/CFF structures and licenses.
- Focused Rust tests:
  `cargo test -p fw-sfnt --test read_otf --test read_containers --test robustness`
  and
  `cargo test -p fw-api --test dispatch cff_import_warning_reaches_document_summary`
  — passed (11 sfnt integration tests plus the filtered API test).
- `cargo fmt --all --check` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed with no
  warnings.
- `cargo test --workspace` — passed; `cargo test --workspace -- --list` lists
  304 Rust tests.
- `pnpm --dir app gen:api && pnpm --dir app check && pnpm --dir app test` —
  passed; 38 Vitest files and 307 tests passed. This conditional block was run
  because two app e2e specifications reference the unchanged DejaVu fixture.
- `cargo run --quiet -p fw-cli -- inspect <new fixture>` — all three opened with
  the documented families/counts/formats/warnings.
- `git diff --cached --check` — passed.
- `rg` found no old fixture name in active `crates`, `app`, `fixtures`, or
  `THIRD_PARTY.md`; the only forbidden-license fixture mention there is the
  existing optional, uncommitted, never-shipped `fontforge-tests` corpus noted
  by policy.

## Concerns

None. The optional external FontForge corpus remains GPL as documented, but it
is absent from the committed fixture set and is never redistributed.
