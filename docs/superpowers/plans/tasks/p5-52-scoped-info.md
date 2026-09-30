# P5 rows 52–53: scoped information

## Scope

Expose the existing font metadata command through a staged font editor, and
make master metric defaults and export instance naming inheritance explicit.
Use only `FontInfo`, `MasterMetrics`, and `Instance` fields already understood
by export and project serialization. Family defaults belong to the current
font document; this task does not create a multi-document family container.

## Ownership

- `MasterMetricsEditor`, metric-default helper/tests, and `InstanceEditor`.
- `FontInfoPanel` integration and focused component coverage.
- Delegated new `FontMetadataEditor`, session, and tests.
- English `scopedInfo` and delegated `fontMetadata` key namespaces.
- Parent owns `DesignSpaceEditor` integration for font defaults.
- Hinting owner retains PostScript zone/stem editing; snapping owner retains
  preferences, grid spacing/subdivision, and keyboard increments.

## Acceptance

1. Unset metric overrides display resolved compiler defaults, with source
   labels, and enabling an override preserves the displayed effective value.
2. Zero overrides remain explicit; resetting an override uses current source
   values rather than stale snapshots.
3. Export instances show effective family/style names and inheritance state;
   an empty override restores the font family or instance style default.
4. Font metadata applies through one existing undoable command, preserves
   unrelated imported fields, participates in pending-edit flush, and retains
   drafts on failure.
5. Focused tests, scoped lint, existing regressions, and an artifact check.
   Parent performs final integrated TypeScript and frontend validation.

## Boundaries

Master source metrics are independently authored, while optional OpenType
metrics derive from them. The current model does not define an external family
document or arbitrary inherited metadata at every scope. Report that limitation
instead of marking the entire family hierarchy complete.
