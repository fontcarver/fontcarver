# P5 PostScript stem analysis quality

Scope: `fc-hint/src/ps.rs`, focused geometric regression tests and an acceptance
report. The hinting owner coordinates separate CID private-dictionary and
variable TrueType foundation work.

Integration guards also cover per-FD analysis and the native Type 1 writer:
single-private-dictionary analysis must reject selected glyphs using another FD,
and Type 1 output must reject unmapped CID private data before replacing output.

The current stem detector accepts a pair of outline edges when one midpoint is
filled. A crossbar can therefore make the full width of a counter-bearing glyph
score above its actual side stems. Write a reproducing regression, then check
cross-sections through each candidate using exact curve intersections and bounded
winding evaluation. Preserve curve-extrema detection and overlapping filled
contours; do not describe the heuristic as complete hinting quality coverage.

Validate geometric invariance under contour reversal and axis exchange, hollow
and crossed counters, and the existing curved-bowl and resource-limit cases.
Use focused Rust tests with the shared Cargo lock and normal profiles. No new
dependencies, source-font mutation, FontForge implementation or executable.

Remaining advanced Type 1 flex/dotsection/counter procedures require explicit
point-bound semantics and are not silently reconstructed from boolean metadata.
