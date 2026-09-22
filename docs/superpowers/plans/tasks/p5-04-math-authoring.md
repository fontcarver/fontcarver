# Editable OpenType MATH

Status: paused at the user's request after the model and standalone codec step.
See [checkpoint and resume checklist](../../../p5-math-checkpoint.md).
MATH authoring and P5 remain incomplete.

Follow the approved font-wide `Font.math` design. Implement all MathConstants,
per-glyph italic corrections, accent attachments, extended-shape flags, four
corner kern functions, horizontal/vertical variants and assemblies. References
use persistent glyph names. Preserve signed device corrections and reject
unsupported device formats explicitly. The current OpenType MATH specification
does not define per-master variation storage for these values.

Source: [OpenType MATH specification](https://learn.microsoft.com/en-us/typography/opentype/spec/math).
Use an original bounded parser and the existing write-fonts offset packer;
the pinned fontations versions have no semantic MATH reader/writer.

1. Add documented model types, validation and additive FCF fields. Add bounded
   semantic binary import/export with correct parent-relative device offsets,
   both coverage formats, reordered glyph IDs and atomic failure behavior.
   Explicit ownership prevents old raw tables from returning after deletion.
2. Integrate export, glyph rename/delete/duplicate, undo/redo and native save.
   Legacy raw MATH remains eligible for the existing preservation policy until
   decoded or explicitly replaced. Malformed imports retain raw data with a
   diagnostic, never partially editable data.
3. Add revision-checked API editing and a bilingual editor with validation,
   usable constant groups, glyph records, variants and assembly preview.
4. Verify synthetic and real font round trips, independent FontTools/HarfBuzz
   interpretation, malformed offsets/counts/devices, allocation/work limits,
   glyph reference changes, undo/save/reopen, export formats and browser flows.
   Run formatting, required crate tests, Clippy and frontend checks.

Do not count a binary foundation alone as finished MATH authoring. Formula
layout beyond assembly preview is not specified by this table and is not part
of this task. Developer comments/docs remain English; UI is English and zh-CN.
