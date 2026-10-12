# P2c-02 Preserve fractional CFF outlines on import

Scope: `crates/fc-sfnt/src/read/cff.rs` and its focused reader tests. Keep the
existing import API; the parent owns CFF writing and compile/export integration.

Replace the rendering-oriented CFF drawing adapter, which truncates fractional
font coordinates, with direct `read-fonts` charstring evaluation. Convert the
16.16 charstring operands to model `f64` coordinates without integer rounding,
and apply the library's effective Top DICT/Font DICT matrix and UPEM adjustment.
Preserve CFF/CFF2 subfont selection, default variation evaluation, charstring
hint detection, and closed-contour endpoint kinds. Do not quantize or increase
roundtrip tolerances to hide data loss.

Acceptance: a failing fractional cubic regression first; synthetic fractional,
nondefault matrix/UPEM, CID subfont and CFF2 default-variation coverage; existing
CFF/TTF/container/robustness reader tests; focused formatting and Clippy checks.
No new dependencies, FontForge execution, or independent commits.

Implementation notes: `CffSource` now evaluates directly into a `CommandSink`,
filters no-op path commands with the existing library adapter, and detects stem
and mask hints in the same pass. `CffPen` applies the library's effective matrix
using `kurbo::Affine`: normalized translation fields hold integer font units in
their raw Fixed bits; the optional scale needs division by 64 to undo the
renderer's 26.6 convention. No charstring coordinate is first converted to f32
or truncated to integer font units. The library remains responsible for matrix
normalization, subroutines, FDSelect and default-coordinate blends.

Original synthetic test tables are assembled in Rust; their CFF2 variation
structures follow the [CFF2 specification](https://learn.microsoft.com/en-us/typography/opentype/spec/cff2)
and [OpenType variation formats](https://learn.microsoft.com/en-us/typography/opentype/spec/otvarcommonformats).
No FontForge source was consulted or adapted.

Validation (2026-09-19): the two initial regressions failed before the change
(`10 != 10.25` and transformed `46 != 48.875`). All seven focused CFF reader
tests now pass. `cargo test -p fc-sfnt` passed all 36 tests present at completion,
including actual Source Han CID, available system TTC, committed fixtures,
robustness and concurrent writer coverage. `cargo clippy -p fc-sfnt --all-targets
-- -D warnings`, owned-file `rustfmt --check`, and `git diff --check` passed.
