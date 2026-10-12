# Type 1 hint replacement

Status: implemented and verified after the static TrueType editor checkpoint `ee30400`.
Advanced Type 1 mechanisms listed below remain open.

Retain standard Type 1 hint substitution and counter-stem metadata at their
actual outline positions. The existing outline evaluator accepts the standard
OtherSubrs 3 sequence in the native regression fixture. Keep its geometry path
unchanged and inspect original programs separately for editable hint state;
never execute arbitrary PostScript or modify the source file. Add an outline
adapter only if a failing geometry regression demonstrates a need.

Map active hint sets to stored contour indices and emit replacement subroutines
from validated point masks. Preserve source sidebearings, matrix normalization,
closing-edge associations and exact outline geometry. Validate active sets before
output replacement. Counter triples must satisfy the Type 1 stem3 constraints.
Do not copy Adobe's restricted OtherSubrs implementation; implement format
semantics with original code and document any interpreter boundaries.

Use native synthetic programs and permissively licensed fixtures, independent
FontTools decoding and native FreeType rendering. Check malformed replacements,
nested subroutines, limits, masks before closing edges, roundtrips, and failed
destination protection. Keep flex, dot sections, arbitrary counter-control
procedures and per-FD dictionaries explicitly open until implemented and tested.

References: Adobe's [Type 1 specification](https://www.adobe.com/content/dam/acom/en/devnet/font/pdfs/T1_SPEC.pdf)
sections 6.4 and 8.1–8.4, and the [Type 1 supplement](https://partners.adobe.com/public/developer/en/font/5015.Type1_Supp.pdf).

Implementation and verification are recorded in
[the hinting checkpoint](../../../p5-hinting-checkpoint.md#type-1-replacement-and-counter-triples).
The bounded metadata scanner, shared point mapping, replacement writer, native
counter triples and original PostScript callback are complete for this scope.
No outline adapter was needed. No model serialization fields or dependencies
were added.
