# FontWorkshop document (`.fwf`)

Version 1 is a UTF-8 JSON document with this envelope:

```json
{
  "format": "fontworkshop",
  "version": 1,
  "font": {}
}
```

`format` is always `fontworkshop`. Readers accept versions up to their current
version and ignore unknown fields for forward compatibility. The optional
`document_path` envelope field is written only for recovery autosaves. Files
named `*.fwf.gz` are gzip-compressed; readers detect gzip from the `1f 8b`
magic regardless of the file extension.

The `font` value uses the serde shapes defined by the public types in
[`fw-model`](../../../fw-model/src/lib.rs). `IndexMap` preserves glyph order,
and binary model fields use the base64 encodings defined by `fw-model`.
Glyph `metrics_keys` (`left`, `right`, `width`, `vertical_origin`, and
`vertical_width`) are existing version-1 model fields and remain persisted
unchanged; resolved metrics and edit-session state are derived and are not
written to the document.

Writers use two-space indentation for the envelope and font-level objects. The
`font.glyphs` map keeps its deterministic `IndexMap` order, but each map entry's
entire glyph value is compact JSON on one physical line. This hybrid layout
makes a glyph edit appear as a focused per-glyph line change in Git instead of
expanding every contour and point across a large multi-line diff.
