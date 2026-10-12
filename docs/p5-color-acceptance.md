# P5 COLR v0 / CPAL acceptance

Verified on 2026-09-26 on Linux with the HTTP development transport and an
isolated headless Chromium session. This block is complete within the static
COLR v0 / CPAL v0/v1 boundaries below. COLRv1, SVG, variable color
authoring and native desktop automation are outside this acceptance.
See [usage](user/color.md) and the [implementation history](p5-color-checkpoint.md).

## Fixes found during acceptance

- UFO private metadata now stores the color payload as JSON text, retaining
  sparse optional label entries that plist cannot represent as nulls. Reading
  also accepts the earlier object representation. Designspace round trips and
  explicit deletion ownership are covered by the same API tests.
- Palette entries and layers display 16 rows per page. Above 256 entries,
  paint controls use an integer field and foreground checkbox; above 256
  palettes, palette selection is numeric. Composition filtering displays at
  most 2,000 matches. These bounds avoid creating excessive repeated options.
- Compiled glyph-order recovery repeatedly scanned post strings through random
  access. It now uses the library's name iterator and still rejects truncated
  tables. A 20,001-glyph reversed-order test failed at 27.883 seconds before the
  fix and passed afterward (entire test 0.25 seconds, lookup budget 2 seconds).
- Real Twemoji edits exceeded Axum's default 2 MiB request limit. The development
  server now has a bounded 64 MiB limit. An HTTP test applies and reads back a
  60,000-entry palette larger than 2 MiB.
- The document language now follows the selected UI language; palette entries
  have a named group role. Existing English and independently translated
  Chinese strings are used. No new dependency or FontForge code was introduced.

## Browser checks

The original geometric fixture was opened through the UI. Invalid RGBA text
survived attempted close and palette navigation. A missing outline name was
rejected and retained on Apply and Close. Correcting it allowed a valid close
to flush the draft. Palette duplication, shared entry insertion, alpha,
foreground and layer order were checked against the actual SVG paths.

Undo restored palette entry 0 to `#D7236975`; Redo restored `#11223388`.
Native Save and reopening after a backend restart retained the edit. The
browser export dialog wrote a TTF; FontTools independently confirmed all three
palettes, four entries, alpha values, and the stack `square:0`,
`bar:foreground`, `triangle:3`.

English/light and Chinese/dark screenshots contain only original geometric
artwork: [palettes](screenshots/p5-color-palettes-en-light.png),
[layers](screenshots/p5-color-layers-zh-dark.png). Captures wait for the
debounced preview to match the selected palette. The dialog scrolls within
the viewport while keeping its footer available.

Twemoji's 1,063-entry palette paged from entries 0–15 to 16–31; filtering found
the 44-layer `u1fab4` composition and its second layer page. The DOM contained
16 entry fields and 16 outline fields. Changing entry 16 from `#6488BDFF` to
`#12345678` through the UI applied successfully; document Undo restored the
original. The external source file was not saved or changed.

The color dialog's targeted WCAG 2 A/AA axe scan reported zero violations.
One contrast check remained incomplete for overlapping/arrow content, which
was inspected visually. This is not a claim of full accessibility conformance.
No JavaScript errors were reported by the browser error collection.

## Independent checks

`scripts/p5_color_acceptance.py` uses FontTools 4.63.0 to compare decoded
COLR/CPAL semantics by glyph ID: RGBA palettes, flags, label IDs and their name
strings, layer order and foreground. It checks baseline and edited TTF,
OTF/CFF, WOFF and WOFF2, native save/reopen, undo/redo and non-mutating preview.
Expected edits are constructed independently with FontTools. OTF cases disable
raw-table preservation while verifying the authored color tables.

The script always generates a small original geometric CPAL v1 fixture with
sparse labels. The external real-font fixture is Twemoji Mozilla v0.7.0:
13,723 glyphs, SHA-256
`6d90152ee0d29e82fe2a87793af5aa4b7ad13e6538360889e141e81ed299ee8e`.
The [upstream release](https://github.com/mozilla/twemoji-colr/releases/tag/v0.7.0)
is external test data; [upstream licensing](https://github.com/mozilla/twemoji-colr/blob/v0.7.0/LICENSE.md)
attributes the art under CC-BY-4.0 and tooling under Apache-2.0. No external font
or its artwork is committed.

Reports are saved incrementally, including the failing operation when a run
does not finish. The initial real-font run exceeded a 120-second WOFF request
limit; a repeated export completed in 63.381 seconds, with about 33.7 seconds
in compilation, 28.4 seconds in preservation/post-processing and 1.2 seconds
in writing. Direct WOFF compression took 0.920 seconds and retained every table.
These debug/shared-host observations are not release performance guarantees.
The final independent run uses an explicit 240-second per-operation budget.

The final run passed for both fonts: **16 verified exports**, plus edits,
native persistence, undo/redo and preview. Saved local evidence is
`/tmp/fontcarver-color-qa/acceptance-final/report.json` (`complete: true`).

| Fixture | Glyphs | Compositions / layers | Palettes / entries | Open | Apply | Save / reopen | Export range |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Original geometric CPAL v1 | 7 | 2 / 5 | 2 / 3 | 46 ms | 3 ms | 11 / 4 ms | 20–27 ms |
| Twemoji Mozilla v0.7.0 | 13,723 | 3,689 / 33,179 | 1 / 1,063 | 14.562 s | 433 ms | 1.784 / 2.659 s | 22.803–39.593 s |

Reproduce with an isolated `fc-devserver` on port 8894:

```sh
python3 scripts/p5_color_acceptance.py --max-operation-seconds 240 \
  --output /tmp/fontcarver-color-qa/acceptance-final \
  /tmp/fontcarver-color-qa/Twemoji.Mozilla.ttf
```

## Verification and boundaries

Final checks:

```text
cargo test -p fc-compile
  164 passed; 0 failed (includes 4 color tests and the post-order budget)
cargo test -p fc-doc -p fc-sfnt -p fc-api --test color
  2 + 6 + 7 passed; 0 failed
cargo test -p fc-devserver
  3 passed; 0 failed
pnpm -C app test
  Test Files 141 passed (141)
  Tests 1146 passed (1146)
pnpm -C app check; pnpm -C app lint
  passed
cargo clippy -p fc-model -p fc-sfnt -p fc-doc -p fc-formats -p fc-compile -p fc-api -p fc-devserver --all-targets -- -D warnings
  passed
cargo fmt --all --check; Prettier on changed TSX; git diff --check
  passed
```

The opt-in external WOFF compression budget test was also run with
`FONTCARVER_COLOR_FONT` set at the previous checkpoint, rather than counting
its early return in the ordinary suite as external coverage. The pre-existing
Tibetan layout expansion-budget failure remains tracked in STATUS; this
acceptance does not claim a green workspace.

CPAL v1 metadata is preserved but has no dedicated metadata editor. COLR v0
layers reference monochrome outlines, including legal self references, rather
than recursive color glyphs. Unsupported/malformed imports retain both raw
tables atomically. Model expansion is bounded; packing supports identical
shared stacks/palettes but may reject complex overlapping ranges that exceed
16-bit writer limits. Grid/text views remain monochrome, and the composition
preview uses the default master with a 1,024-layer cap.
