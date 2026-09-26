# P5 professional finish

Status: resumed on 2026-09-26 for MATH acceptance, now complete within
[its verified boundaries](../../p5-math-acceptance.md). The P5 scope was
authorized immediately after P4; the remaining work is not complete. Current
progress and next actions: [STATUS](../../STATUS.md).
P4 checkpoint: `a3b2f8f`; its supported boundaries and evidence are recorded in
`docs/p4-acceptance.md`.

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
- [x] MATH authoring: constants, glyph data and assemblies; semantic import/export,
      persistence, editor and independent acceptance. See [record](../../p5-math-acceptance.md).
- [ ] Remaining OpenType authoring: COLR/CPAL palettes and layers, SVG documents,
      semantic binary import/export and persistence.
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

Progress and verification for the hinting block are recorded in
[p5-hinting-checkpoint](../../p5-hinting-checkpoint.md); this plan keeps only
the work sequence above.
