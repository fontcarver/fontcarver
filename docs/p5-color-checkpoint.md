# P5 color authoring checkpoint — 2026-09-26

Status: **paused explicitly by the user** on 2026-09-26 after resuming from
`8b1b3d3`. This is implementation and test progress, **not completed color
acceptance**. Branch: `p5-color-authoring`; MATH acceptance is complete at
`6f4ef4f`. Website work is complete except deployment. No push or main merge
has been performed.

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

## External fixture prepared, not yet verified

`/tmp/fontcarver-color-qa/Twemoji.Mozilla.ttf` was downloaded from the upstream
[v0.7.0 release](https://github.com/mozilla/twemoji-colr/releases/tag/v0.7.0):
1,474,284 bytes, 13,723 glyphs, COLR v0 / CPAL v0, SHA-256
`6d90152ee0d29e82fe2a87793af5aa4b7ad13e6538360889e141e81ed299ee8e`.
The font lacks embedded license name records. Upstream
[LICENSE.md](https://github.com/mozilla/twemoji-colr/blob/v0.7.0/LICENSE.md)
attributes the Twemoji art under CC-BY-4.0, with Apache-2.0 tooling. It is an
external, uncommitted test input; do not redistribute it or screenshots of its
art without appropriate attribution. No FontForge tool/code was used.

FontTools 4.63 is installed. A `uharfbuzz.Face` introspection command was still
running at pause and was terminated; **no independent color verification has
run yet**. No acceptance script has been written. The Bungee v2.001 archive and
current Bungee/googlefonts-color-fonts trees did not provide the searched v0
font; no reason to repeat those searches.

## Resume here

1. Read this file, `CONVENTIONS.md` and
   [task scope](superpowers/plans/tasks/p5-05-color-authoring.md). Check git status
   and regenerate API types if needed. Preserve the current implementation.
2. Write `scripts/p5_color_acceptance.py`, analogous to the MATH script, using
   independent FontTools COLR/CPAL decoding. Generate a small original v1 CPAL
   fixture with multiple palettes, sparse entry labels, alpha and foreground;
   also verify the prepared real Twemoji font. Compare records by glyph ID
   because FontTools and FontCarver can synthesize different names for post v3.
   `glyphs.list` supports `sort: "Order"` and bounded paging for the source-name
   to GID mapping. Verify all four containers, edits, undo, FCF save/reopen,
   palette metadata and paint order; inspect source/exports independently.
3. Finish browser acceptance using `agent-browser` in English/light and
   Chinese/dark. The skill stub and CLI `skills get core` were read. The named
   session `fontcarver-color-p5` opened the empty app only, then was closed at
   pause. **No color dialog visual acceptance or screenshots were completed.**
   Do not automate the native desktop. Start isolated servers:
   ```sh
   FC_CONFIG_DIR=/tmp/fontcarver-color-qa/config XDG_DATA_HOME=/tmp/fontcarver-color-qa/data cargo run -p fc-devserver -- --port 8894
   FC_DEV_PROXY=http://127.0.0.1:8894 VITE_FC_API=http://127.0.0.1:1494 pnpm -C app exec vite --port 1494 --strictPort
   ```
   Both servers and their descendants were stopped at the user-requested pause.
   Existing logs under `/tmp/fontcarver-color-qa/` and
   `/tmp/fontcarver-color-{api,codec,ui}-tests.log` are diagnostic only.
4. Review the final diff, run affected-crate regressions and the full frontend
   suite after acceptance fixes, write a user guide and acceptance report,
   update STATUS/audit, and commit. COLRv1/SVG editing remains future work;
   do not mark the entire color-font requirement complete.
