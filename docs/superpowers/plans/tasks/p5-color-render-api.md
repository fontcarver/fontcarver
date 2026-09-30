# P5 color render API

Owner: color_rendering/color_api. Parent owns frontend integration and generation.

## Scope and interfaces

Add `ColorState.preview_color` and `SvgState.preview_svg`, containing validated,
decoded imported data for read-only rendering. Existing `color`/`svg`, status,
ownership, generation and undo behavior remain unchanged. Malformed, unsupported
or unresolvable imported tables expose no preview and retain import errors.

Add optional `ColorGraphPreviewParams.master`, defaulting to the default master.
Compile the isolated static preview from that master's flattened outline geometry
and advances; reject missing masters and missing selected-master layers.

Own only color/SVG API types and handlers plus a focused integration test. Do not
edit generated API files, model/codec code, registries, shared status or commits.

## Acceptance

Write failing tests for imported COLRv0/COLRv1/SVG render payloads, untouched raw
bytes/document/history, malformed imports, and selected-master graph geometry.
Run focused tests under the shared Cargo lock with a task-owned TMPDIR. Format
owned Rust files only. Parent runs API generation, Clippy and broad regression.
Run artifact checks at start and completion; remove owned scratch outputs.
