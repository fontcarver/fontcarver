# P5 Type 42 container

## Scope and ownership

Implement `crates/fc-formats/src/type42.rs` and focused
`crates/fc-formats/tests/type42.rs`; registry and application integration remain
with the remaining-formats/export owners. No dependencies or FontForge source
adaptation. Specification: Adobe technical note 5012, Type 42 Font Format.

## Public interfaces

- `type42::read_bytes(bytes: &[u8]) -> Result<Vec<u8>, crate::Error>` extracts and
  validates one standalone TrueType SFNT from a bounded literal hex sfnts array.
- `type42::write_bytes(sfnt: &[u8]) -> Result<Vec<u8>, crate::Error>` writes a
  Type 42 wrapper with PostScript name, font bounding box, encoding and
  CharStrings derived from the TrueType metadata.

## Boundaries

No PostScript execution, external resources, binary sfnts procedures, arbitrary
computed values, metric overrides, font collections or CFF input. Reader bounds
source/decoded size, token sizes, nesting, and validates embedded table ranges.
Writer preserves SFNT data and uses legal table/glyph string boundaries with
compatibility padding. Unsupported input fails explicitly.

## Acceptance

Tests first: TrueType semantic/byte-preserving round trip, wrapper metadata,
independent hex input, malformed/unsafe/duplicate declarations and invalid SFNT.
Inspect multi-string boundaries. Run focused test and scoped Clippy with shared
Cargo lock and task TMPDIR; parent performs final workspace checks. Resource
checks at start and completion; record actual evidence in acceptance report.
