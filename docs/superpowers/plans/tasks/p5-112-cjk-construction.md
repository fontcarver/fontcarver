# P5-112 — CJK component construction

Scope: provide a glyph-inspector CJK construction section with a document-backed
radical library, bounded Unicode IDS parser and preview, atomic component
construction, and per-component affine transform grid. Use existing
`AddComponent`, `Batch`, and `SetComponent` commands; no model/schema/API changes.
The resulting components remain editable and persist through ordinary `.fcf`.

Ownership: additive `app/src/views/glyph/panels/Cjk*` and `cjk*` modules,
focused tests, English `cjkConstruction` keys, narrow EditInspector insertion
(coordinated with construction owner), dedicated fc-doc integration test.

Acceptance: parser tests cover nested horizontal/vertical/enclosure/overlay IDS,
malformed/oversized input, missing or ambiguous glyphs, and arbitrary glyph-name
leaves. UI tests cover library insertion, preview/apply, stale async results,
error reporting, and transform commands. Document tests exercise atomic failure,
undo/redo and native persistence of generated components. Headless browser
verification uses isolated browser automation if the shared devserver is ready.

Limits: deterministic layout templates are editable starting positions, not
automatic optical design. No external radical-outline assets or remote IDS
database are introduced. Unsupported Unicode description operators fail visibly.

Validation: focused Vitest and a serialized fc-doc integration test under the
shared Cargo lock. Parent runs final workspace/frontend checks. Scratch belongs
only to `.scratch/p5-cjk-construction`; artifact checks run at boundaries.
