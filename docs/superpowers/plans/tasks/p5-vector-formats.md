# P5 vector interchange codecs

Scope: implement SVG 1.1 font source interchange and a bounded EPS single-glyph
outline dialect in `fc-formats/src/vector/`; integration tests in `tests/vector.rs`
and EPS tests owned by the delegated EPS agent. Parent registers the module and
the already-transitive MIT/Apache-2.0 `roxmltree` XML dependency.

Public APIs: `read_svg(path) -> Font`, `write_svg(font, master, path)`,
`read_eps(path) -> Font`, `write_eps(font, master, glyph_name, path)`.

SVG implements a single horizontal font with family, em size, metrics, names,
scalar Unicode assignments and filled outlines. Quadratics become cubic model
contours on import. Unicode aliases use repeated equal named glyph definitions.
Unsupported vertical, contextual, ligature, kerning, painted-child, transform and
external-reference semantics fail explicitly. Source-only metadata, hinting,
layout, extra masters and non-selected layers are not carried by these outline
interchange formats. Writers reject unresolved components and live effects.

Inputs have a 32 MiB byte limit, bounded XML nodes, glyph and path counts,
finite coordinate checks and disabled DTD/entity resolution. Output validation
and full serialization precede atomic replacement. No FontForge code or
execution is used. SVG semantics follow [W3C SVG 1.1 Fonts](https://www.w3.org/TR/SVG11/fonts.html).

Acceptance: failing tests first, source fixture import, fractional geometry and
Unicode/metadata roundtrip, malformed and unsupported inputs, destination
preservation, independent XML/path decode, focused tests and Clippy through the
shared Cargo lock, final artifact check. EPS agent owns EPS implementation and
its tests; parent runs final broad checks after all parallel integration.
