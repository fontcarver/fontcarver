# P5-16 glyph database workflow

Scope: connect the shipped glyph database and document overlay to typed inspection,
revision-checked overlay editing, and atomic Update Glyph Info. Persist overlay in
`font.info.user_data["com.fontcarver.glyphData"]` so existing FontInfo snapshots give
complete undo/redo. Re-derive Unicode, category, subcategory, script and production
name without modifying user outlines or anchor coordinates. Expose anchor and
composition recipes as database suggestions.

Files: new `fc-doc/src/command/glyph_data.rs`, new fc-api glyph_data handler/types
and integration test; coordinated narrow command/dispatch/export registration;
append native schema metadata documentation. Data module and inspector component
are delegated separately.

Interfaces: `Command::UpdateGlyphInfo { glyphs }`; `glyphdata.state` returns an
editable overlay JSON document, effective glyph info and document generation;
`glyphdata.set_overlay` stores validated JSON at a checked generation;
`glyphdata.update` checks generation and applies one undoable selection mutation.

Acceptance: known-name metadata rederives, explicit overlay fields win, variant
suffix inherits classification without duplicating base Unicode, stale calls and
malformed overlays do not mutate, locked selections fail atomically, undo/redo
and native save/reopen retain overlay and resulting metadata. Run focused tests,
formatter and coordinated crate checks; check artifact budget before completion.
