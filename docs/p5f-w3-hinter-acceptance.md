# Real external ttfautohint acceptance

Date: 2026-10-02. Task: [p5f-w3-hinter](superpowers/plans/tasks/p5f-w3-hinter.md).
Recommended row 61 status: **Complete within the previously recorded static-font
scope**. This closes the missing real-executable evidence in limit 9 of the
[hinting acceptance](p5f-k-hinting-acceptance.md); its other limits remain.

## Provenance and isolation

Ubuntu Noble amd64 packages `ttfautohint` and `libttfautohint1t64`, both
`1.8.4-1.1build2`, were downloaded using the host's official Ubuntu package index
and extracted under `.scratch/p5f-w3-hinter/root`. They were neither installed
system-wide nor linked, vendored or bundled into FontCarver. The external program
reported **ttfautohint 1.8.4**.

| Ubuntu package | SHA-256, matching the apt index |
| --- | --- |
| ttfautohint | `124764da91d16ece822c1feb11971b0c8f274f6808d3c34abc656804db36d12a` |
| libttfautohint1t64 | `ecce85bb02e72e99528d25882dc3d5d24ef27dc3a1916d085da3b12563c23a81` |

The host's default apt configuration has conflicting Opera `Signed-By` entries.
Only the existing official Ubuntu source was selected for these downloads; no
system source configuration was changed:

```sh
apt-get \
  -o Dir::Etc::sourcelist=/etc/apt/sources.list.d/ubuntu.sources \
  -o Dir::Etc::sourceparts="$PWD/empty-sources" \
  download ttfautohint libttfautohint1t64
```

This was run from task scratch with an empty `empty-sources` directory, followed
by `dpkg-deb -x` for both downloaded packages. Package identity is documented by
[Ubuntu](https://packages.ubuntu.com/noble/ttfautohint). Expected external-tool
behavior comes from the [upstream manual](https://freetype.org/ttfautohint/doc/ttfautohint.html),
without adapting any external implementation or FontForge source.

## Defect found and fixed

The real `--composites` option adds an internal `.ttfautohint` glyph. The previous
bridge rejected its valid output because `maxp.numGlyphs` changed: DejaVu Sans
went from 6,253 to 6,254 glyphs. Both the new real Rust regression and the CLI
acceptance failed with `ttfautohint: output glyph count changed` before the fix.

The bridge now accepts this single appended helper only for composite hinting,
when neither subglyph adjustment nor dehinting applies. It validates the helper's
`loca` entry, one on-curve point at the origin, nonempty program and exclusion from
`cmap`. Every other count change remains an error. Real execution also confirmed
that `--adjust-subglyphs` overrides the helper-producing composite mode.

Real-binary tests are explicitly ignored in ordinary runs, with instructions to
run them using `--ignored` or `--include-ignored`. An explicit run fails if the
executable is absent. They no longer return successfully after printing a skip.
The independent acceptance script likewise exits 2 with “real ttfautohint must be
on PATH” when the tool is absent.

## Verification

Every Cargo command below was wrapped in `flock .scratch/p5f/cargo.lock`. Tests
used an absolute `TMPDIR` under task scratch. The isolated executable directory
was prepended to `PATH`, and its private `usr/lib/x86_64-linux-gnu` directory was
provided through `LD_LIBRARY_PATH`.

```text
rustfmt --edition 2021 --check crates/fc-hint/src/external.rs \
  crates/fc-hint/src/external/composite.rs \
  crates/fc-hint/src/external/real_tests.rs
  clean
cargo test -p fc-hint -- --include-ignored --nocapture
  unit: 10 passed; PS integration: 9 passed; TT integration: 2 passed
  total: 21 passed, 0 failed, 0 ignored
  both real tests print executable path and ttfautohint 1.8.4
cargo clippy -p fc-hint --all-targets -- -D warnings
  clean
cargo build -p fc-cli
  success
python3 scripts/p5f_real_hinter_acceptance.py \
  --output .scratch/p5f-w3-hinter/cli-after
  6 outputs validated, 5,184 FreeType renders, 18 HarfBuzz comparisons
  3 invalid inputs rejected with destination unchanged; source unchanged
```

FontTools **4.63.0**, FreeType **2.13.2** and HarfBuzz **14.3.0** independently
validated the CLI outputs from a temporary unhinted Noto Sans subset:

| Output | Glyphs | Glyph programs | Renders |
| --- | ---: | ---: | ---: |
| Default TTF | 144 | 131 | 864 |
| Default WOFF | 144 | 131 | 864 |
| Default WOFF2 | 144 | 131 | 864 |
| Composite TTF | 145 | 143 | 864 |
| Adjusted-subglyph TTF | 144 | 141 | 864 |
| Dehinted TTF | 144 | 0 | 864 |

The checks preserve original glyph IDs, `cmap`, horizontal metrics and shaping
for Latin, ligature and accented text; default exports also preserve coordinates.
All sixteen exposed option categories execute across these cases. Invalid ppem,
an unknown script rejected by the real binary, and malformed font bytes leave an
existing destination intact. Fake-process tests still cover timeout/reaping,
invalid output, warning capture, quoted paths and missing-tool behavior.

## Review round and remaining limits

Independent review reproduced corrupt output accepted by the first count-only
exception: a fake tool changed only `maxp`, leaving `loca` short. Helper validation
was added, and `composite_mode_rejects_changed_counts_without_a_valid_helper`
rejects removed, forged extra-one and extra-two counts. The reviewer also caught
fixture paths needing another parent segment after moving real tests into their
own module; those paths are corrected and all 21 tests pass.

This is Linux execution evidence for external version 1.8.4, not native macOS or
Windows evidence or a universal hint-quality guarantee. For an appended helper,
legacy `cmap` formats 2 and 8 are refused because the parser has no mapping
iterator; helper verification is bounded to two million mappings. Unchanged-count
output retains the prior structural validation boundary. No dependencies, i18n
keys or shared status documents were changed. The workspace's other two ignored
tests are fc-api release workflow/performance gates, not external-hinter tests.

After helper validation and the final CLI rebuild, the independent reviewer
rechecked authentic composite output: **145 glyphs, 143 programs, 864 renders and
3 identical shaping samples**. A maxp-only forged output failed with an
out-of-bounds-offset error; a genuine composite font whose `cmap` was changed to
expose the helper as `U+0041` failed with `output glyph count changed`. Both
rejections preserved the existing destination. The review finding is closed.
The six-output acceptance above was completed before that defensive review fix;
the final focused rerun covers its changed composite branch.

## Resource completion

Owned processes are stopped and `.scratch/p5f-w3-hinter` is removed, including the
downloaded packages, extracted executable and generated fonts. Shared build usage
was 14.91 GiB at task start and 15.88 GiB at completion during parallel work;
neither exceeds the 20 GiB ceiling. Final checkpoint:

```text
build: 15.88 GiB, 19,638 files
temporary: 0.02 GiB, 1,134 files
workspace free: 606.06 GiB
warnings: build artifacts exceed 10 GiB; plan profile-cache cleanup
PASS
```
