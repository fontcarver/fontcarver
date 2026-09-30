# P5 AFM metrics interchange

Scope: implement a bounded Adobe AFM 2.0–4.1 horizontal metrics codec in
`crates/fc-formats/src/metrics/{mod,afm}.rs` with integration tests in
`crates/fc-formats/tests/metrics.rs`. Separate helpers own the PFM codec and the
AFM Font bridge. Parent owns crate registration and CLI routing.

Interfaces: `MetricsFont`, `GlyphMetrics`, `Ligature`, `KerningPair`,
`read_afm(&str)`, `write_afm(&MetricsFont)`; re-export bridge functions
`read_afm_path`, `write_afm_font`, `apply_to_font`. AFM values use 1000 units per em.

Preserve supported global properties, character codes, horizontal advances,
optional bounding boxes, ligatures and horizontal kerning. Reject duplicate
identifiers, malformed numbers/counts/framing, unsupported global/character
keywords, vertical/track kerning, composites and multiple-master extensions.
Comments are retained separately. No FontForge implementation is consulted.

Reference: Adobe Technical Note 5004, AFM Specification 4.1,
https://adobe-type-tools.github.io/font-tech-notes/pdfs/5004.AFM_Spec.pdf.

Acceptance: tests first; handwritten AFM records and semantic round trips;
negative malformed/unsupported cases; writer rejection of NaN/injection;
empty metrics sections and bounded input; helper tests cover atomic model
application and non-destructive file export. Focused Cargo tests use shared
flock and task TMPDIR, no broad formatter. Parent runs integrated checks.
