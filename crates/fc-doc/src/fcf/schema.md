# FontCarver document (`.fcf`)

Version 1 is a UTF-8 JSON document with this envelope:

```json
{
  "format": "fontcarver",
  "version": 1,
  "font": {}
}
```

`format` is `fontcarver`. Documents written before the product was renamed
carry the legacy tags `fontcutter` or `fontworkshop`; readers accept both and writers re-stamp
such a document with `fontcarver` on its next save. Readers accept versions up to their current
version and ignore unknown fields for forward compatibility. The optional
`document_path` envelope field is written only for recovery autosaves. Files
named `*.fcf.gz` are gzip-compressed; readers detect gzip from the `1f 8b`
magic regardless of the file extension.

The `font` value uses the serde shapes defined by the public types in
[`fc-model`](../../../fc-model/src/lib.rs). `IndexMap` preserves glyph order,
and binary model fields use the base64 encodings defined by `fc-model`.
Glyph `metrics_keys` (`left`, `right`, `width`, `vertical_origin`, and
`vertical_width`) are existing version-1 model fields and remain persisted
unchanged; resolved metrics and edit-session state are derived and are not
written to the document.

Feature prefixes/blocks and classes carry a persistent string `id`, independent
of their tag, name, code or ordering. New items receive UUID v4 IDs without
separators. Older documents may omit IDs; reading assigns missing IDs and
replaces empty or repeated IDs, preserving the first occurrence of a nonempty
stored ID. Saving persists these identities. Source maps are derived and are
not written to FCF. No draft or active-source revision fields are added by P2b-01.

The older UFO importer stored an entire FEA file as the only feature block with
`tag: "fea"`, `automatic: false` and empty notes. When source authority is `Fea`
and there are no other prefixes/classes/features, reading moves this block to
`prefixes` without changing its ID, text, disabled flag or tag. Other source
arrangements and `ImportedTables` authority are not migrated. Source text keeps
its whitespace, comments and original line endings.

Writers use two-space indentation for the envelope and font-level objects. The
`font.glyphs` map keeps its deterministic `IndexMap` order, but each map entry's
entire glyph value is compact JSON on one physical line. This hybrid layout
makes a glyph edit appear as a focused per-glyph line change in Git instead of
expanding every contour and point across a large multi-line diff.
