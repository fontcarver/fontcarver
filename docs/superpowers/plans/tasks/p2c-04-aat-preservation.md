# P2c-04 Guard AAT and justification preservation during CFF export

Scope: `crates/fc-compile/src/otf_preservation_aat.rs` and its focused
`crates/fc-compile/tests/otf_preservation_aat.rs` integration tests. The parent
owns helper registration and export integration. No commits or broad test runs.

Audit the Apple AAT specifications for `bsln`, `lcar`, `opbd`, `fmtx`, and `kerx`, and
the OpenType `JSTF` specification. Reject preserved contour-point references
with actionable errors, preserve established coordinate formats, and surface
malformed structures as parse errors. Prefer read-fonts typed parsers where
available. Use bounded raw reads for formats without typed parsers. Inspect
the action format in kerx format 4 so coordinate actions remain supported.
JSTF embeds GPOS lookups: until their geometry can be checked, require rebuilding
layout or explicitly disabling original-table preservation.

Acceptance: focused export regressions exercise each rejected point format,
accepted coordinate variants, mixed kerx subtables, malformed/truncated input,
and opt-out recovery. The check must not affect TTF preservation.

References:

- https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6bsln.html
- https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6lcar.html
- https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6opbd.html
- https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6kerx.html
- https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6fmtx.html
- https://learn.microsoft.com/en-us/typography/opentype/spec/jstf

## Result

Implemented `otf_preservation_aat::check` and integrated through the parent.
`bsln` formats 2/3, `lcar` format 1, `opbd` format 1, `fmtx`, and kerx format 4
control-point actions fail with coordinate-rebuild / preservation-opt-out
instructions. Coordinate baselines, carets, optical bounds, kerx formats
0/1/2/6, and kerx format 4 direct-coordinate / `ankr` actions survive unchanged.
The five standard AAT lookup formats 0/2/4/6/8 are exercised. The parent added
the new glyph-indexed tables to the original-glyph-order preservation policy.

Version 2/3 kerx reserved tuple counts are ignored; format 4 ignores tuple count
in every supported version. Actual variation-sensitive kerning requires prior
instantiation. JSTF remains an explicitly bounded limitation: its independent
embedded positioning is rejected until inspected or rebuilt for CFF.

Validation: `cargo test -p fc-compile --test otf_preservation_aat` passed all
16 tests, covering accepted coordinates, rejected point data, malformed headers,
counts, offsets and payloads, retained TTF behavior, and explicit opt-out recovery.
`cargo clippy -p fc-compile --test otf_preservation_aat -- -D warnings` also passed.
