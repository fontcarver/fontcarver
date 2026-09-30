# P5 layer, component and anchor workflows

Scope: close audit rows 32–36 gaps in per-glyph layers, background editing,
component locks and mark previews. Layer backend, component lock semantics and
outline-audit acceptance are delegated independently. This owner edits
LayersPanel, AnchorsPanel, ComponentRow, layerCommands and focused new frontend
helpers/tests. Shared model/API registrations receive coordinated narrow edits.

Behavior: add/duplicate named per-glyph layers, rename layers with persistent undo,
copy selected drawing objects to the corresponding master background and switch
background editing with Mod+B. Anchor matching preview displays available marks
aligned by matching base/underscore anchors; component rows expose persisted
locks. Existing audit behavior receives evidence-driven fixes only.

Acceptance: targeted Vitest command and UI tests, kernel undo/persistence coverage
from delegated owners, frontend check/lint and headless browser acceptance after
integration. User-visible keys are English; independent localization owns Chinese.
No external source adaptation or dependencies. Parent owns shared status and final
broad checks. Each task runs the artifact-budget checker and cleans owned scratch.
