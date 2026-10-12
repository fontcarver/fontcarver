# P5-05: COLR v0 / CPAL authoring

Implement named outline-glyph layers in bottom-to-top paint order and shared
RGBA palettes. COLR v0 and CPAL v0/v1 are supported; CPAL v1 label IDs and
palette flags are preserved. COLRv1 paint graphs and SVG authoring follow later.

Scope: `fc-model`, `fc-sfnt`, `fc-doc`, `fc-compile`, `fc-api`, the color editor
under `app/src/views/color`, Font Info entry, translations, tests and docs.
Use the OpenType [COLR](https://learn.microsoft.com/en-us/typography/opentype/spec/colr)
and [CPAL](https://learn.microsoft.com/en-us/typography/opentype/spec/cpal)
specifications; no FontForge implementation or new dependencies.

Interfaces: `Font.color: Option<ColorTable>` and `color_tables_owned`; validated
undoable `ColorCommand::Replace`; revision-checked `color.state`, `color.apply`,
`color.decode_imported` and outline-based `color.preview` APIs. Glyph references
follow renames, duplication and deletion. Explicit deletion supersedes raw data.
Unsupported or malformed imports retain both original tables atomically.

Acceptance: semantic import/export in TTF, OTF, WOFF and WOFF2; foreground and
alpha; multiple equal-size palettes; valid/malformed binary inputs; glyph order,
production names and excluded references; undo/redo and native save/reopen;
stale API requests; valid draft flushing and invalid-input retention; independent
FontTools decoding and browser visual checks in both themes and languages.

Translation-only delegation, if authorized: edit only the new `color` keys in
`app/src/i18n/zh-CN.json`, translating their completed English counterparts.

## Completion TODO — 2026-09-26

- [x] Model, bounded codecs and undoable revision-checked API.
- [x] Palette/layer editor, independent Chinese translation and bounded controls.
- [x] Native/UFO persistence and semantic TTF/OTF/WOFF/WOFF2 exports.
- [x] Independent FontTools acceptance on original and real-world fixtures.
- [x] English/light and Chinese/dark browser checks, edits, undo and export.
- [x] Regression checks, [user guide](../../../user/color.md) and
      [acceptance report](../../../p5-color-acceptance.md).
- [x] Update the P5 TODOs and stop here as requested; do not start the next task.
