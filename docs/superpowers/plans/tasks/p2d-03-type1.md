# P2d-03 Type 1 interoperability

Implement the Type 1 section of `../../specs/2026-09-19-p2-completion.md`.
Own `crates/fc-formats/src/type1/` and `crates/fc-formats/tests/type1*.rs`.
Public interfaces: `read_type1(&Path) -> Result<Font, crate::Error>`;
`write_type1(&Font, &MasterId, &Path, binary: bool) -> Result<(), crate::Error>`.
Parent owns shared dependencies/registration and API export warnings.
Use `Error::Source { format: "Type 1", reason }`.

Read genuine PFA/PFB via locked read-fonts PS reader. Write actual Type 1
charstrings and encrypted eexec with correct PFA/PFB framing. Cover names,
FontMatrix/UPEM, glyph names/Unicode, advances and cubic curves, component
flattening and exact quadratic raising. Preserve/report encoding and hinting
limitations, reject invalid feature drafts, malformed outlines or dimensions.
Expose a helper for format-loss warnings if useful and report its contract.
No OpenType layout is promised in Type 1 output; parent will disclose that loss.

Acceptance: independent read-fonts and (if available) fontTools/FreeType reading
of both outputs, real permitted external fixture reads, outline/width/UPEM/name
roundtrips, empty glyphs, fractional coordinates, encryption, corrupt input,
failure atomicity. Read conventions/licenses. Focused tests and Clippy. Do not
commit independently.

## Implementation report (2026-09-19)

Implemented native encrypted PFA/PFB import/export in `fc-formats::type1`, with
atomic destination replacement. Cubics and fractional widths are retained;
quadratics are raised before fixed-point quantization; components are flattened
after cycle and expansion checks. FontBBox uses the emitted quantized geometry.
The reader uses an exact Fixed sink and native f64 FontMatrix parsing, preserving
UPEM through 65535, fractional metadata, Notice/copyright and escaped Latin-1.

Native 8-bit encodings are retained. Unicode inference selects one encoded/name
candidate per scalar and leaves unencoded suffix alternates unassigned. The writer,
reader and `loss_warnings` share this policy. Warnings expose stable
`type1-layout-loss`, `type1-hinting-loss` and `type1-unicode-loss` codes.

Supported scope is static, horizontal Type 1 with `hsbw` charstrings. The reader
rejects `sbw` (including subroutines), nonhorizontal FontMatrix advances, Multiple
Master data, stroked PaintType and executable/native metric overrides rather than
silently changing them. To avoid the locked evaluator's large-divisor overflow,
large charstring literals must be adjacent, verifiable integer division operands;
uncertain computed or cross-subroutine large-number arithmetic is rejected.
Output omits hints, layout, anchors and extra source layers.
Limits: 32 MiB/file, 65,535 glyphs, 65,535 bytes/charstring, 100,000 expanded outline
points/glyph and 2,000,000/font; coordinates are bounded to ±16,000 font units and
advances to 0–32,000 units. Fractional operands use portable exact division ratios.

Validation: `cargo test -p fc-formats --test type1` — **19 passed, 0 failed**.
The independent final review also ran the arithmetic guard unit test (**1 passed**)
and native fontTools probes for unsafe division, zero divisors, `sbw`, metric
overrides, transformed matrices, Latin-1 Notice and maximum-width rounding.
Coverage includes fontTools and system FreeType reading both outputs, genuine
external CMBSY10/Nimbus Sans/Feta fonts, corrupt PFBs, P2b alternate-glyph reopen
and `AVfi` shaping, encoding collisions, fractions, large UPEM, transformed bounds,
copyright, unsupported constructs and failure atomicity.
`cargo clippy -p fc-formats --all-targets -- -D warnings` — passed before the final
arithmetic guard additions; the final workspace pass covers those additions. Type 1 files
are rustfmt clean. No FontForge source code was adapted; no font fixtures were
redistributed.
