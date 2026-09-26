# P5 color authoring checkpoint — 2026-09-26

The user requested a durable checkpoint because session usage was nearly
exhausted. **This is implementation progress, not completed color acceptance.**
Branch: `p5-color-authoring`; parent `6f4ef4f` is the completed MATH acceptance
commit. Website work is complete except deployment and needs no development
in this task. No push or main merge is authorized or performed here.

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

## Resume here

1. Read this file, `CONVENTIONS.md` and
   [task scope](superpowers/plans/tasks/p5-05-color-authoring.md). Check git status
   and regenerate API types if needed. Preserve the current implementation.
2. Add API integration tests for revisions, decode legacy raw data, malformed
   and COLRv1 preservation, preview semantics, native save/reopen, deletion and
   export format loss. Verify UFO/designspace private-metadata round trips
   (integration exists but has no dedicated color test yet).
3. Add independent FontTools acceptance script/fixture, analogous to
   `scripts/p5_math_acceptance.py`, checking all exported palette entries,
   foreground/alpha and paint order. Use a real COLR v0 font with verified
   provenance, without committing a large or unlicensed fixture. Add bounded
   repeated-offset/large-font timing tests.
4. Expand UI tests for glyph stacks, reordering, missing names, stale drafts,
   palette switching with invalid input and actual preview geometry. Review
   potentially large layer/palette option lists before claiming large-font UX.
5. Use the `agent-browser` skill for browser-only visual/interaction acceptance
   in English/light and Chinese/dark. Do not automate the native desktop.
   Start isolated servers, e.g.:
   ```sh
   FC_CONFIG_DIR=/tmp/fontcarver-color-qa/config XDG_DATA_HOME=/tmp/fontcarver-color-qa/data cargo run -p fc-devserver -- --port 8894
   FC_DEV_PROXY=http://127.0.0.1:8894 VITE_FC_API=http://127.0.0.1:1494 pnpm -C app exec vite --port 1494 --strictPort
   ```
   Use a named browser session and restore/stop only task-owned processes.
   No dev servers or browser were started for this checkpoint.
6. Finish code review, focused regressions and affected-crate tests, user guide,
   acceptance report and STATUS/feature audit. COLRv1/SVG authoring is future
   work; do not mark the entire color-font requirement complete.
