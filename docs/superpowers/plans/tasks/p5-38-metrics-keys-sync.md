# P5-38 Metrics keys and synchronization

Scope: audit rows 37–38, horizontal formula parsing/evaluation, persistent local
layer overrides, undoable current/all-master update, and Metrics inspector controls.
Owner: metrics_keys subagent under optical_metrics. No commits.

Files: fc-doc metrics resolver and tests; fc-model Layer optional metrics_keys;
narrow Command/apply integration; FCF schema; existing MetricsPanel and resolver
hook; English metrics strings. Parent handles translation and broad verification.

Interfaces: `SetLayerMetricsKeys {glyph, layer, keys}` stores per-layer overrides;
`UpdateMetrics {glyph, master: Option<MasterId>}` resolves formulas before applying
current or all master geometry. No new transport method is required.

Acceptance: syntax cases =n, offsets, mirroring, literal, == local, @ measurement,
auto-aligned relative offsets; invalid/nonfinite/cyclic expressions; local override
isolation; update idempotence/current/all masters, atomic failure and undo; FCF
persistence; frontend component/hook verification. Use shared cargo lock, normal
profiles, focused tests; parent owns final full checks. Report remaining bounds
honestly, especially unsupported external source persistence or platform QA.
