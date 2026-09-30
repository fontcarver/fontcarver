# P5 standalone proof export

Owner: proofing/proof_export; coordinated by `p5-resume-coordination.md`.

Scope: implement `app/src/views/text/proofExport.ts`, its focused tests, and
`docs/p5-proof-export-acceptance.md`. Parent owns UI, downloads, and i18n.

Public interface: `ProofEntry` contains label, already-positioned PreviewGlyph
outlines, unitsPerEm, and size. `exportProofHtml(title, entries)` returns standalone
HTML; `exportProofPdf(title, entries)` returns standalone PDF bytes. Both accept
readonly entries and reject malformed or nonfinite geometry. Preserve all shaped
x/y positioning and quadratic/cubic contours. Kernel paths use M/L/Q/C/Z.

HTML preserves Unicode labels, escapes untrusted strings, embeds SVG outlines,
and requires no installed source font or network assets. PDF uses vector paths,
A4 pagination, and fits oversized runs into the printable area. Base PDF fonts
cannot display every script: label characters outside printable ASCII display
as U+XXXX, with original Unicode in ActualText and PDF title metadata. Proofed
outlines themselves preserve every script without depending on installed fonts.

Acceptance: tests first for escaping, transforms, geometry bounds, quadratic to
cubic conversion, pagination, valid object offsets, malformed inputs, and Unicode
label preservation. Run focused Vitest, file-specific ESLint/Prettier, and a
Poppler render inspected visually. Parent runs shared frontend checks after
integration. Use `.scratch/p5-proof-export` with absolute TMPDIR and clean owned
scratch after retaining the report and necessary visual evidence.
