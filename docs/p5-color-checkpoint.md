# P5 color authoring checkpoint — 2026-09-26

Status: **COLR v0 / CPAL acceptance complete** on 2026-09-26. Work stopped at
the user's requested task boundary after updating the TODOs. See the final
[acceptance report](p5-color-acceptance.md) and [user guide](user/color.md).
The checkpoints below are historical, including their then-unfinished checks.
Branch: `p5-color-authoring`; MATH acceptance is complete at `6f4ef4f`.
Website work is complete except deployment. No push or main merge was performed.

## Saved implementation

- Font-wide `ColorTable`: COLR v0 named monochrome outline references, ordered
  bottom to top; equal-size RGBA palettes, foreground sentinel; CPAL v0/v1
  metadata retention (flags, palette labels, entry labels).
- Bounded binary codecs with atomic import fallback for unsupported COLRv1 or
  malformed data; edited/deleted data owns both tables. Shared identical
  palettes/stacks are packed once. More complex overlapping original ranges
  are expanded and may exceed writer packing limits; errors are explicit.
- TTF/OTF/WOFF/WOFF2 export against actual glyph order. Excluded layers are
  omitted with a warning; source data is unchanged. Selecting COLR also keeps
  required CPAL. Deleted tables cannot be restored from imported bytes.
- Undoable replacement, rename/duplicate/delete reference maintenance, native
  FCF/FCF.gz persistence, UFO private metadata integration, format-loss warnings.
- APIs: `color.state`, `color.apply`, `color.decode_imported`, `color.preview`.
  Revision checks prevent stale replacement. Preview flattens monochrome
  outlines only, including legal self-references; limit 1024 layers.
- Font Info → Color fonts dialog: palette duplication/removal, shared entry
  add/remove with index maintenance, hex RGBA and picker, glyph stacks with
  ordering/foreground choice, SVG preview, apply/discard and save handover.
- English strings and an independently translated Chinese `color` namespace.
  The user explicitly authorized this translation-only agent. No other work
  was delegated; no new dependencies or FontForge code were introduced.

`Layer.color_layers` remains the older opaque embedded-outline container; it
is not the new named COLR v0 representation. CPAL v1 metadata is retained,
not yet editable through dedicated label/flag controls. Grid/text previews
remain monochrome; the new color dialog has the composition preview.

## Verified so far

- `cargo check -p fc-api`: passed.
- `pnpm -C app gen:api`: passed (generated API files stay uncommitted).
- `cargo test -p fc-doc -p fc-compile -p fc-sfnt --test color`: **10 passed**.
  Covers codecs, corrupt tables, all four export containers, CPAL labels,
  production/source names, excluded references, native persistence, ownership,
  rename/duplicate/delete, undo, and table selection.
- `pnpm -C app check` and `pnpm -C app lint`: passed.
- Frontend regression: **141 files / 1141 tests passed**, including four new
  color editor/model tests. The invocation `pnpm -C app test -- <paths>` ran the
  full suite; use `pnpm -C app exec vitest run <paths>` for focused runs.
- `cargo clippy -p fc-model -p fc-sfnt -p fc-doc -p fc-formats -p fc-compile
  -p fc-api --all-targets -- -D warnings`: passed.
- `cargo fmt --all`, Prettier on new frontend files and translations, and
  `git diff --check`: passed.
- Full workspace Rust tests are not claimed green; STATUS tracks the
  pre-existing Tibetan layout budget failure.

## Second checkpoint: API coverage and editor bounds

- Added `crates/fc-api/tests/color.rs`: **7 tests passed** for atomic revisions,
  native save/reopen and export/deletion, undo/redo, foreground/alpha/paint-order
  previews, legal self references and ignored nested COLR, legacy provenance,
  unsupported/malformed raw preservation, format-loss warnings, UFO/designspace.
- These tests exposed an actual UFO bug: optional CPAL entry-label array items
  cannot be represented as plist nulls. UFO now stores `color_json` as a JSON
  string inside the private metadata; readback also accepts the earlier
  `color` object. Explicit deletion ownership survives both UFO and designspace.
- Added codec expansion and timing tests: **6 tests passed**, including a
  20,000-glyph semantic round trip and rejection of shared-range amplification.
- Added UI tests: **8 tests passed**, covering layer reorder/foreground/opacity,
  invalid text across palette/glyph changes, stale draft rejection, and large
  palette/layer controls. Large stacks show 16 rows per page. Above 256 entries,
  paint controls use a checked integer field plus foreground checkbox instead
  of repeating thousands of options. Above 256 palettes, selection is numeric.
- Independent translation agent added five Chinese keys, matching all 57
  English color keys and placeholders. Authorization from the user persists
  for translation-only work; do not delegate unrelated work.

Final pause checks passed: `pnpm -C app check`, `pnpm -C app lint`,
`cargo fmt --all --check`, `git diff --check`, and
`cargo clippy -p fc-api -p fc-formats -p fc-sfnt --all-targets -- -D warnings`.

The previous checkpoint's 1141-test full frontend run is historical. This
second checkpoint reran the 8 color UI tests; it has not rerun the full suite
or completed browser/independent-engine acceptance.

## External fixture

`/tmp/fontcarver-color-qa/Twemoji.Mozilla.ttf` was downloaded from the upstream
[v0.7.0 release](https://github.com/mozilla/twemoji-colr/releases/tag/v0.7.0):
1,474,284 bytes, 13,723 glyphs, COLR v0 / CPAL v0, SHA-256
`6d90152ee0d29e82fe2a87793af5aa4b7ad13e6538360889e141e81ed299ee8e`.
The font lacks embedded license name records. Upstream
[LICENSE.md](https://github.com/mozilla/twemoji-colr/blob/v0.7.0/LICENSE.md)
attributes the Twemoji art under CC-BY-4.0, with Apache-2.0 tooling. It is an
external, uncommitted test input; do not redistribute it or screenshots of its
art without appropriate attribution. No FontForge tool/code was used.

FontTools 4.63 is installed. No HarfBuzz color verification is claimed. The
Bungee v2.001 archive and current Bungee/googlefonts-color-fonts trees did not
provide the searched v0 font; no reason to repeat those searches.

## Third checkpoint: partial independent and browser acceptance

The user requested another pause on 2026-09-26. Saved work since `a14c1e9`:

- Added `scripts/p5_color_acceptance.py`. It generates an original geometric
  CPAL v1 fixture with two palettes, sparse name labels, alpha and foreground.
  FontTools independently verified import semantics, baseline and edited
  TTF/OTF/WOFF/WOFF2 exports, referenced name strings, native save/reopen,
  undo/redo and read-only preview. All eight synthetic exports passed.
- For Twemoji, import and baseline TTF/OTF semantics passed. The first baseline
  WOFF request exceeded the script's 120-second limit, so the full run failed
  and **no complete `report.json` was produced**. Edited real-font exports and
  WOFF2 remain unverified. The script currently writes its report only after
  every font passes; preserve partial evidence when improving it.
- A separate repeat WOFF export completed in **63.381 seconds** in the debug
  server. Progress timestamps put approximately 33.7 seconds in compilation,
  28.4 seconds in preservation/post-processing and 1.2 seconds in final writing.
  This does not establish a WOFF compression defect: direct compression took
  **0.920 seconds**, and decompressed tables matched. The cause of the earlier
  timeout is still unconfirmed; do not claim a performance fix.
- Added an opt-in compression budget integration test in
  `crates/fc-compile/tests/webfont.rs`, using `FONTCARVER_COLOR_FONT` and a
  30-second assertion. It passed with the externally generated baseline TTF.
- Actual browser checks passed for invalid RGBA retention across close/palette
  navigation; apply; palette duplication; shared entry insertion; alpha,
  foreground and layer reorder preview; and native Save As. Saved UI document:
  `/tmp/fontcarver-color-qa/ui-accepted.fcf`.
- Saved English/light and Chinese/dark screenshots using only the original
  geometric fixture: [palettes](screenshots/p5-color-palettes-en-light.png) and
  [layers](screenshots/p5-color-layers-zh-dark.png). Wait for the debounced SVG
  preview to update before taking further screenshots.
- Observed an accessibility issue: Chinese UI still leaves `<html lang="en">`.
  No fix made yet; inspect the language effect in `app/src/shell/AppShell.tsx`.
  Browser undo/redo, reopen, export, invalid glyph drafts and large-font editor
  controls still need final acceptance.

Evidence under `/tmp/fontcarver-color-qa/`: `independent.log`,
`profile-export.log`, `profile-export.mjs`, `acceptance/0-color-acceptance/`,
`acceptance/1-Twemoji.Mozilla/`, and `profile.woff`. These are local diagnostics,
not committed artifacts. The profile WOFF completed but has not been separately
compared with FontTools. No product-code change was made in this third session.
Pause checks: acceptance script parses; Rust formatting and diff whitespace
checks pass. No new full-suite or clippy run is claimed for this checkpoint.

## Final completion and next task boundary

- Fixed quadratic compiled post-name lookup, the development transport's 2 MiB
  request limit for large color drafts, document language and palette group role.
- FontTools verified all 16 baseline/edited exports across two fixtures. The
  final report is `/tmp/fontcarver-color-qa/acceptance-final/report.json`, with
  `complete: true`. It records the explicit 240-second operation budget and
  per-format timings; real-font exports completed in 22.803–39.593 seconds.
- Finished browser invalid-input, close-flush, undo/redo, save/reopen, export,
  language/theme and large palette/stack paging checks. Saved screenshots show
  the settled preview for the selected palette, with validation notices dismissed.
- Frontend: 141 files / 1,146 tests passed. `fc-compile`: 164 tests passed.
  Color API/doc/codec: 7 + 2 + 6 passed. Development HTTP server: 3 passed.
  Type checking, lint, clippy, Rust/TSX formatting and whitespace checks passed.
- User guide, acceptance record, STATUS, feature audit and task TODOs updated.
  Static COLR v0/CPAL row 104 is complete within the documented boundaries.

The named browser `fontcarver-color-p5` and isolated servers on ports 8894/1494
were closed at completion. Local evidence and the original geometric UI file
remain under `/tmp/fontcarver-color-qa/`; preferences there are English/light.
No native desktop automation, release packaging, remote push or main merge.

**Stop here per the user's instruction.** SVG color documents and COLRv1 are
queued future tasks; begin one only after the user resumes. Existing known
workspace limitations, including the Tibetan layout budget failure, remain
tracked in STATUS. No need to repeat completed MATH or color acceptance.
