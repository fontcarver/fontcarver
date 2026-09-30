# P5 — Virtual master coordinates and sparse sources

Owner: variable/CJK coordinator, resumed 2026-09-30.

## Scope

Add named virtual master coordinates as source presets in the design-space
editor. A virtual master is a location without global metrics, kerning or
full-font glyph layers. A glyph can explicitly duplicate its current source at
that location as a brace source; untouched glyphs remain sparse. Coordinates
are copied into sources, so later preset edits never silently move geometry.

Model: `VirtualMaster { name, location }`, `Font.virtual_masters`, optional
`VariationState.virtual_masters`, atomic `DesignSpaceCommand::VirtualMasters`
and `VirtualSource` using the existing revision-checked variation API.
Validate finite mapped coordinates, nonempty unique names, unique locations
apart from physical masters, maximum 1024 positions. Axis renames migrate
virtual positions; removing a referenced axis fails without dropping metadata.

Files: model font/designspace modules, document designspace command and impact,
FCF schema, variation API, API generator import list, independent
VirtualMastersEditor and DesignSpaceEditor mounting, English keys. No status
edits, no generated file edits, no source-format interchange claims.

## Acceptance

Write failing tests for metadata-only addition, atomic invalid input, creating
only the selected glyph's brace source, undo/redo, native save/reload, and
axis-rename migration. Verify interpolation and variable export use explicit
sparse source geometry. UI tests cover staged save and preset source creation.
Focused test commands use the shared Cargo lock and task TMPDIR; parent runs
final broad checks. Check artifact budget before/after tests and completion.
