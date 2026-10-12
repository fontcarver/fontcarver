# P5 MATH acceptance

Verified on 2026-09-26 on Linux, using the HTTP development transport and an
isolated headless Chromium session. The MATH authoring block is complete within
the boundaries below; the rest of P5 remains open. Implementation history is in
[the checkpoint](p5-math-checkpoint.md); usage is in
[the user guide](user/math.md).

## Fixes found during acceptance

- Invalid text could disappear when closing or changing sections/records, and
  document handovers could apply the last parsed value. Close and navigation
  now validate first. Save/close handovers also validate unparsed text, including
  when the closing document has suspended and disabled its form. Required glyph
  names are checked even in that disabled state. Explicit discard still works.
  Six component tests cover these paths and a combined constants/glyph edit.
- DejaVu Math TeX Gyre's radical top has a 502-unit start connector and a 287-unit
  full advance. The previous validator rejected the whole MATH table because
  the connector exceeded the advance. The
  [OpenType GlyphPart definition](https://learn.microsoft.com/en-us/typography/opentype/spec/math#glyphpart-record)
  specifies them independently. Removed that extra restriction while retaining
  finite/range, reference, count and resource checks. A synthetic codec test
  verifies round-trip preservation and bounded preview layout.

No dependencies or font binaries were added, and no FontForge implementation
was used or executed.

## Browser workflow

DejaVu Sans was opened through the file dialog. From Font Info, the MATH editor
was used to set Axis height to 777, add corrections `12: -2 0 3`, and set
`parenleft`'s assembly italic correction to 17. The combined edit was applied;
the selected glyph stayed selected. Undo restored Axis height 642, and Redo
restored the edit. Save As wrote a native FCF document; reopening it retained
the values. The export dialog wrote a TTF, independently decoded with FontTools
to confirm the constant, device deltas and assembly edit.

Invalid numeric text blocked closing and switching sections while remaining
visible. Both English/light and Chinese/dark views were inspected. No
horizontal overflow was found at the 1280-pixel viewport and no browser
JavaScript errors were reported. A 16,384-unit parenthesis request produced
seven parts and an actual 16,860-unit assembly; the UI reports that result
rather than claiming an exact fit.

DejaVu Math TeX Gyre was also opened in the browser after the connector fix.
Its radical construction reached 8,000 units with nine parts. An added record
for a missing glyph produced an explicit error, retained the draft and could
be discarded without changing the font.

Screenshots:
[constants](screenshots/p5-math-constants-zh-dark.png),
[assembly](screenshots/p5-math-assembly-zh-dark.png),
[radical](screenshots/p5-math-radical-en-light.png).

## Independent verification and performance

`scripts/p5_math_acceptance.py` runs the same API as the editor. FontTools 4.63.0
compares every decoded MATH field; HarfBuzz 14.3.0 via uharfbuzz 0.56.0 compares
all 56 constants, minimum overlaps, italics, accents, extended-shape flags,
four-corner kern records, both directions of variants, and assembly parts.
Glyph IDs are resolved to names. Comparisons run at 0, 12, 13, 14 and 24 PPEM to
include signed Device corrections.

Each font gets an unchanged TTF round trip, edited constants and an assembly,
a non-mutating preview, native save/reopen, undo/redo, and edited TTF, OTF/CFF,
WOFF and WOFF2 exports. Expected edited data is constructed independently with
FontTools. DejaVu's unrelated GPOS point anchors cannot survive quadratic-to-CFF
conversion: the OTF cases explicitly disable preservation of raw imported
tables. Owned MATH still survives and is compared in full.

| Font                      | Glyphs | MATH records |    Open | Read MATH | Apply | Preview |   Save / reopen |          Export |
| ------------------------- | -----: | -----------: | ------: | --------: | ----: | ------: | --------------: | --------------: |
| DejaVu Sans 2.37          |  6,253 |           60 | 1.797 s |   5–16 ms | 31 ms |    5 ms | 2.776 / 4.310 s | 11.572–27.275 s |
| DejaVu Math TeX Gyre 2.37 |  4,282 |        2,682 | 1.023 s |  67–73 ms | 15 ms |    2 ms | 0.905 / 1.110 s |   4.121–4.410 s |

These are single-run debug-build timings on a shared machine, with Rust tests
running concurrently; they are regression evidence, not release benchmarks.
The development server's peak RSS for this run was 321,736 KiB (314.2 MiB).
All requests met the script's 120-second budget; peak server memory met its
1,024-MiB budget. The larger font is an external system file recorded in
[fixtures/README.md](../fixtures/README.md), not a committed fixture.

Reproduce with a freshly started development server, using its PID for the
optional Linux memory check:

```sh
FC_CONFIG_DIR=/tmp/fontcarver-math-config \
XDG_DATA_HOME=/tmp/fontcarver-math-data \
cargo run -p fc-devserver -- --port 8894

python3 scripts/p5_math_acceptance.py \
  --api http://127.0.0.1:8894 --output /tmp/fontcarver-math \
  --server-pid PID \
  fixtures/fonts/DejaVuSans.ttf \
  /usr/share/fonts/truetype/dejavu/DejaVuMathTeXGyre.ttf
```

The output directory contains native documents, exported fonts and `report.json`
with versions, timings and memory usage.

## Regression checks

| Command                                                                             | Result                             |
| ----------------------------------------------------------------------------------- | ---------------------------------- |
| `pnpm -C app check`                                                                 | Passed                             |
| `pnpm -C app lint`                                                                  | Passed                             |
| `pnpm -C app test`                                                                  | 140 files, 1,137 tests passed      |
| `cargo test -p fc-model -p fc-sfnt`                                                 | 131 passed, no failures or ignores |
| `cargo test -p fc-api --test math -p fc-compile --test math`                        | 11 passed, no failures or ignores  |
| `cargo clippy --workspace --all-targets -- -D warnings`                             | Passed                             |
| `cargo build -p fc-devserver`                                                       | Passed                             |
| `cargo fmt --all --check`, changed-file Prettier, Python syntax, `git diff --check` | Passed                             |

The six new dialog tests and five existing MATH model/session tests are included
in the 1,137 frontend total. The new connector regression is included in the
131 Rust total. Their failure cases were reproduced before fixing them.

One full frontend run hit an unrelated TextView test timing failure: its spy
observed an absent active editor just after the glyph appeared. That test passed
in isolation, and the next full run passed all 1,137 tests with the default
configuration. No TextView code or test was changed; this transient remains
visible here rather than being counted as a MATH defect.

## Scope boundaries

This verifies the MATH editor and supported semantic data. It does not certify
native desktop input, complete mathematical-expression typesetting, or all
platforms. Variable/static-instance and production-name paths have Rust
integration coverage; the independent real-font corpus here is static.
VariationIndex corrections remain unsupported and are retained as raw imported
data with a diagnostic. Assembly previews use equal repeat counts for extenders
and are limited to 1,024 parts. Font-wide MATH values do not vary by master.

The full workspace test suite is not claimed green: the previously recorded
Tibetan layout expansion-budget failure is outside this acceptance block.
