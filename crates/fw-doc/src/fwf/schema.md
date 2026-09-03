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
