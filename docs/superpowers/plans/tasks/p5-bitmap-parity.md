# P5 bitmap parity integration

Scope: audit rows 64–71 and 101–103, retaining existing strike editing and export paths. Integrate separately owned tracing modes, integer UPM grid presets, bitmap reference editing, canvas tools/HUD, PNG sheets and conservative pixel-font detection. Add atomic cross-strike copy/nearest rescale with depth conversion, pixel-as-outline-components using a persistent editable `pixel` glyph, and bounded atomic sheet glyph updates.

Owned files: bitmap API types/handlers, bitmap command additions, BitmapView/BitmapEditor integration and new bitmap transfer/reconstruction UI. Shared registry changes are narrow. Other task owners retain unrelated edits. No model schema change or new dependencies.

Acceptance: first add regression tests for cross-strike pixel placement/metrics/depth, pixel-component reuse, transaction failure, undo/redo/native persistence; use focused Rust/API and frontend tests. Trace/sheet/canvas owners provide independent pixel tests. Headless agent-browser visual acceptance with existing devserver, no global desktop automation. Parent runs final workspace checks. Resource checker runs at task boundaries; Cargo uses shared flock and default space-saving profiles. Evidence records unsupported cases honestly.
