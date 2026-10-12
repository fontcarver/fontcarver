# P5-60 static CFF subroutinization

## Scope

Add opt-in static CFF1 subroutinization to the native writer and OTF export.
Deduplicate complete, byte-identical Type 2 programs into Global Subrs. Preserve
widths, curves, stems, masks, glyph order, and CID Font DICT selection. Retain
the original programs when INDEX overhead would erase the saving. This first
implementation does not extract shared fragments or optimize CFF2/TrueType/Type1.

## Files and interfaces

- `fc-sfnt/src/write/subroutinize.rs`: internal deterministic extraction helper.
- `fc-sfnt/src/write/mod.rs`: public writer export and private module registration.
- `fc-sfnt/src/write/cff.rs`: coordinated owner adds
  `write_cff_with_subroutines(font, master, order, production_names, subroutinize)`;
  existing `write_cff` remains unchanged by default.
- `fc-sfnt/tests/cff_subroutinize.rs`: semantic and size regression tests.
- `fc-compile/src/otf.rs`: pass the option through both initial and reordered CFF
  writes. Export-options owner wires the caller and unsupported-format guard.

## Acceptance

Tests first: compressed repeated programs are smaller and fontations evaluates
the same geometry, widths, stems, and masks, including CID dictionary selections.
Test unprofitable programs remain byte-identical, deterministic output, and
subroutine index encoding across the 1,240-entry bias transition. Use the shared
Cargo lock and ordinary profiles for targeted tests and crate Clippy. Parent owns
final workspace checks. Check artifact budgets before and after test work.

Clean-room implementation from Adobe Type 2 specification §2.3 and §4.4:
https://adobe-type-tools.github.io/font-tech-notes/pdfs/5177.Type2.pdf
No FontForge source, external subroutinizer, or new dependency is used.
