You are working in the FontWorkshop repository (Rust workspace + React app), main tree, branch `p1-outline-editing`. Task: **replace three test-fixture fonts that are under GPL/AFPL licenses with permissively licensed equivalents**, keeping test coverage identical. Read `CONVENTIONS.md` (especially the "Licensing and porting from FontForge" section), `docs/licensing.md`, `THIRD_PARTY.md` (fixtures section) and `fixtures/README.md` first. Do not touch `.worktrees/`. Do not dispatch other agents.

## The three files to remove (use `git ls-files fixtures` to find their exact paths)
1. `CantarellMin.woff2` — 2009 Cantarell release, GPLv3+ (README wrongly says OFL). Used as the WOFF2 container fixture.
2. `test1012.otf` — FontForge GPL-3 test data. Used by fw-sfnt OTF/CFF reading and robustness tests.
3. `NimbusRoman-Regular-o.otf` — AFPL/GPL. Used as a CFF OpenType fixture.
Find every use: `grep -rn "CantarellMin\|test1012\|NimbusRoman" crates app fixtures docs THIRD_PARTY.md` (known: crates/fw-api/tests/dispatch.rs, crates/fw-sfnt/tests/{robustness,read_containers,read_otf}.rs, fw-fixtures helpers). Read each test to learn exactly which features the fixture exercises (WOFF2 decoding; CFF charstrings/private dict/encoding; name/cmap/OS2 fields; malformed-input robustness derived from a valid base; specific glyph names/counts asserted).

## Replacements (must be SIL OFL 1.1, Apache-2.0, or Bitstream Vera licensed; verify from the font's own name table IDs 13/14 or the upstream LICENSE, never guess)
- Tooling available: Python 3 with fontTools 4.63 and brotli (so `pyftsubset` / `fontTools.ttLib` can subset, and write WOFF2). No system OTF fonts are installed; network access is available for downloading upstream releases (GitHub releases of Adobe Source Sans 3 / Source Serif 4 / Source Code Pro are OFL and ship CFF `.otf`; Noto CJK OTF is OFL if a CJK CFF case is wanted; DejaVu is already in the repo under the Bitstream Vera license).
- WOFF2: build from an OFL/Vera font already in the repo or downloaded (e.g. subset DejaVuSans or Source Sans 3 to ~50 glyphs and save with `flavor = "woff2"`). Name it descriptively (e.g. `SourceSans3Min.woff2`).
- CFF OTF #1 and #2: subset two different OFL CFF fonts (e.g. Source Sans 3 Regular and Source Serif 4 Regular) to a small glyph set that still covers what the tests assert (keep any glyphs the tests name; keep hint/private-dict content; do not flatten to TrueType). Keep each fixture well under 2 MB (target < 300 KB); record the exact upstream URL, version/tag, the subsetting command, and the license in `fixtures/README.md`.
- If a test relied on a property only the old file had (e.g. a specific quirk of test1012.otf), reproduce that property in the new fixture with fontTools (document how) or, if impossible, adjust the assertion to an equivalent check on the new file and explain in the report.

## Required edits
- `git rm` the three files; add the new ones (binary, `git add`).
- Update all tests' paths, glyph counts, names and any asserted values to the real values of the new fonts (compute them, do not guess); keep every test meaningful.
- `fixtures/README.md`: replace the three rows (source, version, license, subset command); `THIRD_PARTY.md`: update the fixtures table accordingly (all fixtures must now be OFL/Vera/Apache); include the OFL notice text for OFL fonts (the OFL requires the license text to accompany the font — add `fixtures/LICENSE-OFL.txt` with the full OFL 1.1 text and, per font, the Reserved Font Name statement from its upstream license).
- Verify: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `pnpm --dir app gen:api && pnpm --dir app check && pnpm --dir app test` if any app test references fixtures.
- Commit on the current branch with a Conventional Commit, e.g. `test(fixtures): replace GPL/AFPL test fonts with OFL-licensed subsets`. Do not push. Do not rewrite git history.

## Report
Write the full report to `.superpowers/sdd/2026-09-03-p1-outline-editing/fixtures-relicense-report.md` (per fixture: upstream, license evidence, subset command, size; per test: what changed; verification output summary). Final message under 12 lines: status, commit hash, test summary, concerns.
