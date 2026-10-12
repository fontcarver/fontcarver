# P5 color render API acceptance

Implemented optional `ColorState.preview_color` / `SvgState.preview_svg` for
validated imported data. Existing editable fields, ownership/status and imported
errors retain their semantics. `color.graph_preview` accepts an optional master,
uses its outlines/advances, and rejects nonexistent masters or missing layers.
No variable COLRv1 decoding, model changes, generated-file edits or commits.

Added four integration regressions in
`crates/fc-api/tests/color_render_state.rs`: imported COLRv0/COLRv1/SVG payloads,
unchanged document/raw bytes/generation/history, malformed or unresolved imports,
selected-master geometry/advance, and missing-layer rejection. Tests were written
before implementation, but lock queueing prevented observing an assertion-level
red run. Independent read-only review found no concrete defects.

Verification:

- Scoped `rustfmt --edition 2021` and `git diff --check`: passed.
- `cargo test -p fc-api --test color_render_state`: blocked by unrelated concurrent
  compile errors in fc-compile, fc-formats and fc-doc, subsequently fixed by owners.
- Retry `cargo test -p fc-api --test color_render_state --test color --test svg`:
  blocked by unrelated `DocId` Display, smart-filter `GlyphName.as_str`, and
  cross-paste `LayerId` Ord errors. Final output:

```text
Some errors have detailed explanations: E0277, E0658.
For more information about an error, try `rustc --explain E0277`.
error: could not compile `fc-api` (lib) due to 9 previous errors
```

Neither run reached test execution; runtime acceptance remains pending the root's
final fc-api suite and generated-TypeScript integration. Both commands finished;
no owned process remains. Per root's stop instruction, no further builds run here.
Owned scratch outputs removed; artifact checker passed at start and completion.
