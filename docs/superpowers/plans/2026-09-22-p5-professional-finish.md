# P5 professional finish

Status: active batch integrated and stopped on 2026-09-30 at the user’s request.
See [batch checkpoint](../../p5-resume-integration.md). MATH, COLR v0/CPAL
and the static SVG/COLRv1 authoring subsets have acceptance records. The P5 scope was
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
- [x] COLR v0 / CPAL authoring: palettes, layers, persistence, four export
      containers, independent FontTools and bilingual browser acceptance.
      See [record](../../p5-color-acceptance.md).
- [x] Static SVG documents: source/bindings editor, bounded codecs, persistence
      and four-container independent acceptance. See [record](../../p5-svg-acceptance.md).
- [x] Static COLRv1 paint graphs, semantic binary import/export and persistence.
      See [record](../../p5-colrv1-acceptance.md); variable paints remain raw.
- [x] Batch style effects: eight editable presets, captured selection/master preview,
      atomic undo/backups and independent native/export acceptance. See
      [record](../../p5-style-effects-acceptance.md).
- [x] Non-destructive layer effects: uniform strokes, overlap removal, stored
      boolean masks, source-preserving editing, bake/undo and independent export
      acceptance. See [record](../../p5-layer-effects-acceptance.md).
- [ ] Geometry and construction: variable pen points and smart
      geometry, raster-image trace, background images, optical spacing/kerning, CJK
      construction helpers and remaining editor/metrics/variable requirements.
      Numeric live smart corners are complete: signed radii, source-node editing,
      local smart interpolation, bake/undo/native persistence and independent
      export checks. See [record](../../p5-smart-corners-acceptance.md);
      radius drag handles remain in the interaction backlog.
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
integration. The Tibetan layout expansion-budget failure was diagnosed and fixed on
2026-09-28: import conversion eligibility now applies the compiler budget gate,
while the read-only lookup inventory and original tables remain available.

## Hinting implementation checkpoint

Progress and verification for the hinting block are recorded in
[p5-hinting-checkpoint](../../p5-hinting-checkpoint.md); this plan keeps only
the work sequence above.
