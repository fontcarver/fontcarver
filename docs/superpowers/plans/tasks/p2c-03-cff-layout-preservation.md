# P2c-03 Check geometry-dependent retained layout before CFF export

Scope: `crates/fc-compile/src/otf_preservation.rs` and
`crates/fc-compile/tests/otf_preservation.rs`. The parent owns registration,
export integration, outline writing, and shared preservation policy changes.

Converting TrueType outlines to CFF changes the numbering and meaning of
outline points. Inspect retained GPOS, GDEF and BASE data with read-fonts
before conversion. MATH has only glyph IDs and coordinate/device values, so
the existing glyph-order preservation policy is sufficient for it. Reject
structures that reference original contour points
with an actionable error; accept coordinate-based positioning and ordinary
device adjustments. Parse errors must remain errors, rather than hiding an
uninspected point-dependent structure. Check extension positioning too.

Acceptance: focused synthetic write-fonts regressions cover rejected outline
point references, accepted coordinate positioning, extension lookups, absent
tables, and malformed retained layout. Report outline-specific table and
variation preservation recommendations to the parent. No commits, broad test
suite, or edits to the parent's modules.

## Additional vertical-metrics correctness scope

The parent also assigned `crates/fc-compile/src/otf_vertical.rs` and focused
integration tests. Quadratic and cubic bounds can differ, so regenerate
`vmtx` top sidebearings, `vhea` aggregate bounds-derived values, and `VORG`
using the final CFF bounds. Preserve compiled advances. Prefer selected-master
modeled origins; when absent, recover the compiler's effective origin from
the compiled TrueType `yMax` and top sidebearing. Emit current per-glyph origins
instead of retaining an imported VORG that could be stale after edits.

Acceptance: vertical advance and origin preservation, fractional cubic bounds,
edited imported origins, empty glyphs, absent vertical metrics, and invalid
numeric ranges. Parent remains responsible for helper registration/integration.

## Verification

- `cargo test -p fc-compile --test otf_preservation --test otf_vertical`:
  20 layout guard tests and 9 vertical metrics tests pass.
- `cargo clippy -p fc-compile --test otf_preservation --test otf_vertical -- -D warnings` passes.
- The layout guard checks declared array byte ranges explicitly because the
  generated read-fonts accessors can return empty arrays for truncated data.
- Coordinate anchors/carets/baselines and ordinary Device adjustments retain
  their original bytes; point indices and uninstantiated variable layout return
  actionable errors. Existing TTF preservation and explicit opt-out still work.
- Vertical tests cover actual cubic extrema, edited imported origins, compiler
  fallbacks, empty glyphs, OpenType rounding, large valid advances, and overflow.

References: [MATH coordinate/device structures](https://learn.microsoft.com/en-us/typography/opentype/spec/math)
and [VORG explicit vertical origins](https://learn.microsoft.com/en-us/typography/opentype/spec/vorg).
The separate AAT/JSTF extension is tracked in `p2c-04-aat-preservation.md`.
