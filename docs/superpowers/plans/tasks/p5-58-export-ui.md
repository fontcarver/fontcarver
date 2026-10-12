# P5 export workflow UI boundary

Implement local durable named export profiles with validated, versioned storage;
reuse profile snapshots in a bounded named-instance/profile matrix with unique
output paths and individual success/failure reports; add explicit preflight
checks and glyph navigation. Extend the existing export-only processing UI for
extrema, rounding, and decomposition once the API owner generates bindings.

Owned files: ExportDialog/ExportProcessingOptions and new export profile,
matrix, preflight modules and focused tests. English-only keys under
`exportProfiles`, `exportMatrix`, `exportPreflight`, plus narrow processing
option additions. No generated bindings, export format registry, translations,
or shared status edits. Parent owns final broad checks.

Work split: profile persistence/picker and preflight navigation are independently
delegated. The UI owner implements shared request snapshots, matrix validation
and execution, integrates panels, and runs focused Vitest/ESLint plus isolated
headless visual checks if available. Acceptance includes storage remount,
invalid-storage handling, path collisions, partial batch failures, snapshots,
and preflight navigation without mutating document state.
