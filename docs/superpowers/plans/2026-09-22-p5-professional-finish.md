# P5 professional finish

Status: in progress. Authorized by the user immediately after P4, without further
confirmation. P4 checkpoint: `a3b2f8f`; its supported boundaries and evidence are
recorded in `docs/p4-acceptance.md`.

## Scope and completion rule

Implement the P5 row of the approved design and close every Must-have and
Should-have row in `docs/p5-feature-audit.md`. Do not replace implementation with
placeholder controls, silently drop requested data or call an unverified native
platform complete. Preserve English developer documentation and comments, English
and Simplified Chinese UI, and final formatting. No FontForge execution or copied
implementation is used. Native/global desktop input automation remains prohibited
by the earlier user instruction; headless browser testing and builds are allowed.

## Work sequence

- [ ] Hinting: bounded TT assembler/disassembler, editable programs/CVT/maxima,
      source-index validation, hinted preview, PS stems/zones/masks, CFF/Type 1 output,
      external ttfautohint bridge with honest availability/results.
- [ ] OpenType authoring: MATH constants/glyph data/assemblies, COLR/CPAL palettes
      and layers, SVG documents, semantic binary import/export and persistence.
- [ ] Geometry and construction: style effects, non-destructive effects and smart
      geometry, raster-image trace, background images, optical spacing/kerning, CJK
      construction helpers and remaining editor/metrics/variable requirements.
- [ ] Document workflows: compare/family comparison, merge, paste special,
      cross-document copy, visual diffs and reference binary handling.
- [ ] Automation: generated TypeScript API, scripts console/reload, Python RPC,
      plugin lifecycle/manager, reporter/filter preview contracts and documentation.
- [ ] Formats/export: remaining SVG/EPS/AFM/PFM/TTX/collection paths, functional
      preprocessing options, stored profiles, preflight and shared CLI behavior.
- [ ] Shell/UX: complete checklist gaps in windows/workspaces, filters, metadata,
      proofing, shortcuts, accessibility/help, feature editing and previews.
- [ ] Release: Linux deb/AppImage and macOS dmg/Windows msi build definitions,
      locally verifiable installers, platform CI, user/developer docs.
- [ ] Final row-by-row acceptance, independent font decoding/rendering, realistic
      corpora, failure/undo/save/reopen tests, formatting and regression checks.

## Initial audit

P4 implements variable/CJK paths. `fc-hint` is still absent; export currently emits
explicit warnings for autohint and remove-overlap requests. PS/TT data structures
exist, but there is no program editor or authored-program export. MATH and colour
font data are currently opaque imported tables. `fc-formats` currently exposes UFO,
Glyphs, SFD, Type 1 and bitmap formats. Release configuration has generic Tauri
bundle defaults and no platform release workflow. These are real implementation
gaps and remain open until verified.

## Verification strategy

Prefer focused tests for behavioral changes, then required crate/frontend checks.
Use independent FontTools/FreeType/HarfBuzz checks for emitted font structures and
rendering. Test invalid/truncated/oversized binary input and stale revisions.
Exercise user workflows in an isolated headless browser with the agent-browser
skill, preserving the user's active desktop. Final broad checks run after feature
integration. The pre-existing Tibetan layout expansion-budget failure is tracked
separately and must be diagnosed during professional finish rather than hidden.

## Hinting implementation checkpoint

The first slice adds `fc-hint`, exact TT opcode assembly/disassembly and bounded
external ttfautohint execution. PS editing supports stems, ghosts, masks, counter
masks, closing edges, zones and private values through revision-checked API
transactions, undo and FCF persistence. Curve extrema supplement straight-edge
stem analysis. Static CFF writes these values; static export preprocessing now
supports overlap removal and automatic hinting through both UI and CLI.

Focused verification found and corrected mask point offsets, implicit closing
edge masks, cross-master hint invalidation and missing curved-bowl stems.
Invalid active sets containing overlapping stems are rejected at CFF export.
Independent FontTools/FreeType acceptance produced 2,460 successful glyph/size
renders and 125 TT glyph programs for each of TTF, WOFF and WOFF2; decoded web
containers match the TTF rendering digest. WOFF2 was decoded by FontTools because
the host FreeType cannot load that container directly. A separate browser export
added 30 renders and verified the editable source remained unchanged.

This is not the full hinting milestone. Complex Type 1 hint import/output needs
further implementation. Static-master TT editing, persistence, compiled-point
checks, authored program export and interpreter preview are now integrated. Variable
hinting, multiple editable CID private dictionaries and broader PS analysis
quality remain open. The full P5 checklist is still open.

The Type 1 follow-up retains literal private values, H/V stems, ghosts, fractional
operands, point-bound hint replacement and representable stem3 counter groups.
Import normalizes sidebearings and representable matrices; runtime private-field
computation and unsupported glyph mechanisms produce persistent diagnostics.
Native output validates every active set and rejects unsupported mechanisms
before replacing a file. The real Nimbus Sans fixture verifies unsorted BlueValues,
nonzero sidebearings and replacement hints. FontTools, FreeType and Ghostscript
checks cover both PFA and PFB output. Flex, dot sections, general counter control,
seac hint inheritance and per-FD dictionaries remain open.
PFA/PFB automatic hinting and overlap removal now run on export copies through
the API, CLI and bilingual export dialog. Old PS hints cleared without rehinting
are disclosed; failed preprocessing preserves the existing destination.

The TT follow-up adds a separate bilingual editor for glyph/fpgm/prep assembly,
CVT and interpreter maxima, with actual compiled point indices and phantom points.
Revision-checked apply, undo/redo, save/reopen, stale-binding failures and native
TTF/WOFF/WOFF2 export are verified. WOFF2 null transforms preserve zero-contour
programs that the ordinary encoder discards. Smart/bracket resolution, generated
instances, variable TT programs and simultaneous automatic hinting/overlap removal
remain unsupported for authored TT programs. See `docs/user/hinting.md` for exact
interpreter and source-resolution limits.
