# P2 completion: static CFF and source interoperability

Date: 2026-09-19. Status: implemented and verified within the boundaries below.
Final evidence: [P2c/P2d acceptance](../../p2cd-acceptance.md).

The user requested completion of all remaining P2 work. P2a/P2b remain the
accepted baseline. This design covers P2c and P2d, following the binding product
design and `CONVENTIONS.md`. Performance investigations, FontForge comparison,
native desktop input automation and release packaging are outside this work.

## P2c: static OpenType/CFF

Emit a name-keyed CFF1 table with Type 2 charstrings from the selected master's
model outlines. Resolve components using the existing bounded, cycle-detecting
outline resolver. Raise quadratic segments to cubic exactly; preserve cubic
control points, with explicit numeric encoding limits and bounded rounding.
Write empty glyphs and `.notdef`, widths, charset, strings, dictionaries and a
valid FontMatrix for the actual UPEM. Reject invalid names, nonfinite geometry,
open contours and unrepresentable values with actionable errors.

Reuse the compiler's exact glyph-ID order and its metadata/layout tables. Replace
the TrueType outline-dependent tables, emit `OTTO`, CFF `maxp` 0.5 and `post` 3,
and recompute bounds/metrics where the outline flavor changes. Do not retain TT
programs, stale variation data or incompatible outline-dependent tables. Preserve
eligible imported layout only with verified glyph identity/order, retaining the
P2b source-ownership and unapplied-draft guards. Report hint loss explicitly.
Variable CFF2, CID output, PS autohint and subroutinization remain later work.

OTF is available through the shared export API, CLI and existing export dialog.
Existing WOFF/WOFF2 defaults continue to produce TrueType-flavored fonts.

## P2d: native source formats

Each importer reads actual native structures and each writer emits usable native
structures. Private metadata may retain extra editable information, but must not
replace native outlines, widths, components, anchors, kerning or feature rules.
External edits to native fields take precedence over stored metadata. Unknown
source data is retained where safe; unsupported behavior that would be lost or
misinterpreted is explicitly rejected or reported, never silently discarded.
All readers bound input and validate indices, counts, recursion and numeric data.
All writers validate before touching the destination and use atomic file writes.

### Glyphs

Read Glyphs 2/3 and write Glyphs 3 ASCII property lists. Cover names, UPEM,
masters, axes/locations, glyph ordering/export flags, Unicode assignments,
cubic/quadratic paths, components, anchors, groups/pairs and FEA source. Keep
unknown dictionaries/custom parameters in a raw source shadow. Use the already
locked permissive `glyphs-reader` dependency where useful. Treat unsupported
smart/conditional/color behavior explicitly. Preserve model extras in userData
without allowing stale extras to overwrite external edits.

### SFD

Read/write static SFD, including font information, encodings, glyph names,
metrics, cubic/quadratic splines, references/transforms, anchors and expressible
kerning/layout. Support the format's text encoding. Preserve safe unknown fields;
reject unsupported structures that affect output. A source-only roundtrip must
not hide omitted layout inside an opaque metadata payload. SFD support does not
claim the separate SFDir, bitmap, variable or CID milestones.

### Type 1

Read PFA/PFB via `read_fonts::ps::type1`, carrying names, units, Unicode mapping,
widths and cubic outlines into the model. Write static PFA and PFB using native
Type 1 charstrings, encrypted charstrings/eexec and correct PFB segment framing.
Use the selected master and flatten components. Report the format's lack of
OpenType layout, source layers and retained hints before output; do not imply
that Type 1 can preserve GSUB/GPOS. Invalid source drafts remain export blockers.

## Find problems v1

Provide a deterministic, read-only document scan with stable diagnostic codes,
severity and locations (glyph/master/layer where applicable). Cover duplicate
or invalid Unicode, missing master layers, missing/cyclic components, malformed
contours/nonfinite geometry, invalid metrics, kerning references/membership and
feature validation. Bound returned results, expose truncation and bind results
to document generation. Expose through API, CLI and a keyboard-accessible UI;
selecting a glyph issue navigates to its glyph. Editing invalidates stale results.
Repairs use existing explicit editor commands; no unsolicited auto-fix occurs.

## Acceptance

Test structural output with independent readers, semantic outline/metric
roundtrips, real font fixtures and shaping comparisons. Test malformed input and
failure atomicity. Source roundtrips must cover external native-field edits and
unknown metadata. Exercise CLI and isolated real-backend browser workflows,
English/light and Chinese/dark UI. Run relevant Rust/frontend suites, generated
API checks, formatting, Clippy, TypeScript, lint and production build. Record
actual coverage, exclusions and commands in P2 acceptance before marking complete.

## Primary format references

- Adobe CFF and Type 2 specifications: https://adobe-type-tools.github.io/font-tech-notes/
- Glyphs 3: https://github.com/schriftgestalt/GlyphsSDK/blob/Glyphs3/GlyphsFileFormat/GlyphsFileFormatv3.md
- SFD: https://fontforge.org/docs/techref/sfdformat.html
- Type 1: Adobe Type 1 Font Format and the locked fontations reader implementation.

## Implementation plan

Status: complete, 2026-09-19. Acceptance: [P2c/P2d](../../p2cd-acceptance.md).
This plan lived in `plans/` until 2026-09-24 and was merged into its design.

| Task | Scope                                                               | Owned implementation                                         |
| ---- | ------------------------------------------------------------------- | ------------------------------------------------------------ |
| C1   | CFF1 writer, charstrings, sfnt conversion, independent-reader tests | `fc-sfnt` write modules; `fc-compile` OTF integration        |
| D1   | Glyphs 2/3 importer, Glyphs 3 writer, passthrough and roundtrips    | `fc-formats/src/glyphs/`, format tests                       |
| D2   | SFD importer/writer and native semantic roundtrips                  | `fc-formats/src/sfd/`, format tests                          |
| D3   | Type 1 PFA/PFB reader/writer and roundtrips                         | `fc-formats/src/type1/`, format tests                        |
| D4   | Structured problem scanner, bounded query and CLI                   | model/doc scan modules, API problem types/handler, CLI check |
| I1   | Shared format guards, API/CLI import-export and UI integration      | shared registration, export types/handlers, existing dialogs |
| I2   | Independent Chinese localization, browser verification and review   | locale, end-to-end tests, acceptance evidence                |

Implement and verify format kernels before exposing their targets. Keep shared
registration/dependency edits coordinated. Tests must demonstrate native
interoperability rather than only decoding our own metadata. Complete scoped
reviews and an integrated regression gate before updating milestone status.

### Progress

- [x] Audit P2 scope, baseline and reusable dependencies.
- [x] Record design and implementation sequence.
- [x] C1 static OTF/CFF, fractional reader, safe table preservation and vertical metrics.
- [x] D1 Glyphs.
- [x] D2 SFD.
- [x] D3 Type 1.
- [x] D4 Find problems, including independent review regressions.
- [x] I1 API, CLI and UI integration.
- [x] I2 Sol localization, independent review and final acceptance (8/8 browser workflows).
