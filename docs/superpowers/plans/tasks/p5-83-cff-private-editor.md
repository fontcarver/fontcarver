# P5-83 CID CFF private dictionary editor

## Scope and ownership

Expose imported CID font dictionaries in the existing PostScript hint editor,
allow appending and editing a dictionary, and assign the selected glyph layer to
one of the master's dictionaries. FD0 remains `Master.private_dict`; FD1 onward
use `Master.cff_private_dicts`. Scope includes hint commands/state, command impact
routing, narrow PS editor components, English hint strings and focused tests.
Model fields, CFF import/export and FCF schema belong to the CFF preservation
agent. Generated API definitions are generated through the existing generator.

## Public interface

- `HintCommand::CffPrivate { master, index, private }` validates before replacing
  an existing dictionary or appending the next dictionary (maximum 256 total).
- `HintCommand::CffPrivateAssignment { master, glyph, layer, index }` assigns an
  existing dictionary to a layer in that master. Zero clears the optional index.
- Hint state includes CID availability and the selected layer's dictionary index.
- Both commands use the existing revision-checked atomic `hints.apply` path.

## Acceptance

Focused API tests demonstrate independent FD edits, assignment, missing FD and
wrong-master rejection, atomic rollback, revision rejection, undo/redo and FCF
save/load. UI tests demonstrate selecting/editing FD1 without changing FD0,
adding a dictionary, changing glyph assignment and retaining draft on failure.
The root owner runs final workspace and frontend integration checks. Budget
checks are mandatory at start and completion; Cargo commands use the shared lock.
