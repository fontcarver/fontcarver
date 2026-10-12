# P5-103 pixel-outline detection and strike reconstruction

## Scope and ownership

Own `crates/fc-raster/src/pixel.rs` and focused reconstruction tests. Coordinate
additive exports with the tracing agent. No sfnt, API, model or frontend changes;
the bitmap parent integrates detection and reconstruction into those workflows.

## Interface and behaviour

- `detect_pixel_font(&Font, &MasterId)` returns a conservative optional
  `PixelFontDetection` with ppem, font-unit grid size and evidence counts.
- `reconstruct_pixel_font(&Font, &MasterId, &[String])` returns the optional
  monochrome strike after the same detection, with existing raster bounds and
  metrics guarantees.
- Inspect all master glyphs, require finite closed orthogonal outlines and a
  common origin-aligned grid at an integer ppem between 4 and 256. Flatten plain
  components only after bounded expansion checks; reject effects and curves.
- Require three geometrically distinct glyphs with repeated unit-grid
  staircases. Rectangle-only fonts and repeated copies do not establish a
  pixel-font suggestion. Exact integer-grid data is supported; arbitrary
  near-grid outlines, rescaled/rounded sfnt geometry, and photographic tracing
  are outside the conservative detector's contract.
- Bound font glyph/outline work, coordinate magnitudes and raster pixel work.

## Acceptance

Write failing tests first. Recover ppem and exact bitmap pixels/metrics for a
synthetic traced font; cover components, holes/negative origins, empty glyphs,
floating font-unit grid, unrelated rectangles, repeated evidence, diagonal and
curved outlines, off-grid metrics, invalid inputs and resource limits. If an
existing sfnt round-trip harness can be used without new dependency coupling,
verify exported/reimported geometry there; otherwise parent owns that test.

Use `.scratch/p5-pixel-reconstruction/` and shared
`.scratch/p5-resume/cargo.lock`; format owned files and run focused fc-raster
tests/Clippy. Parent owns final workspace checks. Check artifacts at boundaries
and delete only this task's scratch data. No commits.

## Implementation and acceptance status — 2026-09-30

Implemented the public detection/reconstruction API in `fc-raster/src/pixel.rs`.
The tracing owner added its exports. Detection checks finite source data before
flattening, bounded component expansion, complete master alignment and unique
staircase evidence. Evidence ignores translation, contour ordering, start-point
rotation, redundant straight points and contour winding. Reconstruction uses
the existing bounded monochrome rasterizer and preserves its metric handling.

Eight integration tests were written before implementation: exact pixels and
metrics, holes and negative origins, fractional font-unit grids, components,
rectangle/clone rejection, curve/diagonal/off-grid rejection, malformed/cyclic
inputs, expansion limits, equivalent contour encodings and a timing budget.

The focused command was queued under the shared Cargo lock, then canceled at
the parent's instruction during shared Rust integration repairs:

```text
cargo test -p fc-raster --test pixel_reconstruction
INTERRUPTED before execution; neither the red run nor final tests passed.

rustfmt --edition 2021 --check crates/fc-raster/src/pixel.rs crates/fc-raster/tests/pixel_reconstruction.rs
exit 0

python3 scripts/check_artifacts.py
build: 6.07 GiB, 13,160 files
temporary: 0.10 GiB, 7,335 files
workspace free: 669.03 GiB
PASS
```

The full helper module compiled in the tracing owner's initial run before the
public wrapper was connected; this is compile evidence only. Final functional
tests and Clippy remain with the parent's coordinated Rust integration batch.
No sfnt round-trip fixture was added because sfnt files belong to other owners
and this crate has no sfnt dependency. No dependencies or FontForge code were
added. Detection intentionally omits fonts lacking three independent staircases,
near-grid rounded outlines, sizes outside 4..=256, and live/smart geometry.
