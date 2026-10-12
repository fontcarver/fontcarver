# TrueType hint authoring

Status: the static-master editing slice is implemented and verified. Advanced
program-aware source resolution and variable hinting remain open. Binary
foundation checkpoint: `f15d0bb`; integration evidence is recorded in
`docs/p5-hinting-checkpoint.md`.

## Behavior

Expose glyph assembly, CVT, fpgm, prep and interpreter maxima through a separate
TrueType editor. The PS editor must remain usable when imported TT bytecode is
malformed. Display actual compiled point indices, including off-curve and phantom
points. Provide an isolated hinted preview at selected pixel sizes.

Authored glyph instructions bind to the compiled outline's point identity and
metrics. Recompilation must validate this binding before attaching the program.
Geometry changes leave authored source available for correction; export must
reject a stale binding rather than attach it to different points. Revision checks,
atomic apply, undo, redo and FCF persistence cover both glyph and master changes.

Imported programs may be promoted only when their source point identity agrees
with the compiled basis. Otherwise, show them separately as reference and start
the compiled-point draft empty. Authored programs are distinct from the existing
option to retain imported instructions. Document the treatment of automatic
hinting, overlap preprocessing, variable fonts and generated static instances.

## Binary implementation

- Build a bounded reader for simple/composite point identity, component
  dependencies, metrics and a versioned digest. Detect cycles and excessive work.
- Patch instruction spans in existing glyph bytes, preserving outline encoding;
  rebuild loca and update head/maxp. Support zero-contour phantom-point programs.
- Replace CVT/fpgm/prep and validate maxima, bytecode size and malformed pushes.
- Reuse existing layout point-index guards whenever authored output uses a new
  compiled outline order with preserved imported tables.
- Keep native TrueType interpreter preview bounded, and surface interpreter
  errors instead of silently falling back to an unhinted rendering.

## Verification

Test nontrivial programs on simple, composite and zero-contour glyphs; phantom
point numbering; changed source and component geometry; changed metrics; invalid
CVT/program/maxima values; truncated input; revision conflicts; failed export
destination protection; undo/redo/save/reopen; and exact decoded output programs.
Use independent FontTools decoding and FreeType rendering, plus headless UI QA.
The primary format references are Microsoft's [glyf specification](https://learn.microsoft.com/en-us/typography/opentype/spec/glyf),
[maxp specification](https://learn.microsoft.com/en-us/typography/opentype/spec/maxp)
and [instruction reference](https://learn.microsoft.com/en-us/typography/opentype/spec/tt_instructions).
