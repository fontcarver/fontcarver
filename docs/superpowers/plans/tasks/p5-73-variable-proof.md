# Variable text proofing

Scope: read-only axis and named-instance text proofing for TextView parity rows 73/75.

Owned files: dedicated fc-api types/handler/tests named variable_proof; app text VariableProofPanel, useVariableProof, CSS and tests. Parent integrates TextView/i18n; root owns shared Rust dispatch/module/API registry integration.

API: text.variable_preview receives doc, text, user-space location, optional named instance index, features/script/language/direction; returns generation, user-space location, positioned SVG paths and advances. Instantiate the complete font, shape it outside the document lock, and bound caching to a single current document/revision/location and one latest text response. Never mutate source masters.

UI: collapsed by default, axis sliders and named-instance selector, actual interpolated outline preview. Debounce requests and reject stale replies. No source-master switching.

Acceptance: midpoint advances, outline geometry, interpolated kerning, named-instance mapped coordinates, rejection of invalid coordinates; stale response rejection and hidden panel request suppression in Vitest. Focused Cargo commands use shared flock and task TMPDIR. Parent performs integrated broad checks and visual verification. Resource checks at task start/end, no new dependencies.

Implementation completed: exact static instantiation, shaped positioned SVG outlines, one-entry snapshot/response cache, collapsed panel with mapped named instances/sliders, and stale-response guards. Parent integrated TextWorkspaceControls and English keys; rust_integration integrated shared Rust/API export registries. Narrow Rust formatting and Prettier completed.

Acceptance execution handed to root after the explicit host-load pause on delegated test/build processes. Required filters: `cargo test -p fc-api --test variable_proof`; `pnpm exec vitest run src/views/text/useVariableProof.test.ts src/views/text/VariableProofPanel.test.tsx`. These tests are not claimed passed until root records results. Parent owns integrated typecheck/lint/visual acceptance. No new dependencies or FontForge source adaptation; task scratch removed.
