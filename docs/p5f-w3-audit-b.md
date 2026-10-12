# Independent final audit: rows 57–112

Reviewed 2026-10-02 against the shared integrated checkout. This is a read-only
source and evidence review, not a new execution of the 56 workflows. Root owns
the final integrated test barrier; concurrent wave-3 changes require their own
acceptance and review. Existing acceptance evidence is reused only where the
current implementation retains the asserted behavior.

The user's latest instruction defers publication. Signing credentials are not
configured; unsigned local build and installation verification remain in scope.
Neither a CI definition nor a successful cross-compile counts as native runtime
or installer evidence.

## Findings requiring integration decisions

1. **Row 71: native PNG saving lacks the application's native artifact path.**
   [BitmapSheet](../app/src/views/bitmap/BitmapSheet.tsx) creates a Blob URL and
   exposes only an `<a download>` link. [saveArtifact](../app/src/shell/saveArtifact.ts)
   provides native save-dialog selection and scoped `writeFile` for proof sheets,
   but the bitmap sheet does not use it. No Tauri download handler was found in
   `src-tauri`. This does not prove every native WebView rejects the link; it
   does mean browser evidence cannot establish the native PNG save workflow.
   Route the explicit Download action through the shared helper and cover native
   delegation, cancellation and failure. Preserve the preview and browser
   download behavior. The bundled Chromium's independently reproduced download
   cancellation is a separate environment problem.
2. **Row 111: “all proofing surfaces” exceeds the implemented vertical scope.**
   [BitmapWaterfall](../app/src/views/bitmap/BitmapWaterfall.tsx) advances only
   `x += g.advance`; its `bitmap.preview` request has no direction. Outline
   previews, variable previews, detached text previews and PDF/HTML proofs do
   carry vertical layout. A bitmap waterfall is nevertheless a proofing surface.
   Either implement and verify vertical bitmap proofing or explicitly narrow the
   row's accepted scope; the literal requirement is currently Partial. Merely
   passing the external large-CJK test would not close this issue.
3. **Row 112: reproduced stale component overwrite during asynchronous Apply.**
   [CjkTransformGrid](../app/src/views/glyph/panels/CjkTransformGrid.tsx) captures
   `components` and `drafts`, awaits `activeEditor()?.settle()`, then sends indexed
   `SetComponent` commands. [applyCommand](../app/src/editor/editSession.ts)
   sends `doc.apply` without a generation guard. The render-time signature clears
   newly displayed drafts when components change, but cannot invalidate an
   already suspended Apply closure. A component replacement/reorder during the
   await can therefore apply a captured component to a changed index. This is a
   source-derived risk was reproduced by the parent-authorized regression below.
   Clicking Apply, replacing the component while settle is pending, and releasing
   the barrier sends the old component over the replacement at the same index.
   The dedicated CJK fix owner must protect the suspended commit against actual
   identity/generation changes and retain the regression.
4. **Rows 61, 62, 71, 109: execution evidence is being refreshed elsewhere.**
   The prior real-`ttfautohint` test returned early when the program was missing;
   a reported unit-suite pass did not prove that binary ran. Row 62 still needs
   actual installed-package checks and native macOS/Windows evidence. The three
   browser download cases affect PNG and PDF/HTML end-to-end evidence. Link the
   new owner reports and exact outcomes rather than overwriting historical
   results with an unconditional green claim.

Findings 1–3 were sent to root during review. Dedicated bitmap and CJK agents
own their fixes. This audit adds only the authorized regression for finding 3.

## Per-row review

“Retain” means the existing bounded Complete claim is supported by the inspected
code and meaningful prior evidence, conditional on the final integrated gate.
It does not mean a new browser or independent-font execution was performed here.
“Review” requests the focused resolution above before final sign-off.

| Row | Recommendation                                          | Current implementation and checked evidence                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| --- | ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 57  | Retain                                                  | [GlyphVariationPanel](../app/src/views/glyph/panels/GlyphVariationPanel.tsx) keys the preview by document/glyph and publishes an overlay; [VariationPreview](../app/src/views/fontinfo/VariationPreview.tsx) debounces `variation.preview`, rejects stale responses, exposes axis-unit slider steps, numeric input and reset. Read-only scrubbing is distinct from source edits. [Export acceptance](p5f-f-export-acceptance.md).                                                   |
| 58  | Retain                                                  | [ExportProfiles](../app/src/dialogs/ExportProfiles.tsx) loads project profiles; [export handler](../crates/fc-api/src/handlers/export.rs) stores them through font-info commands and resolves matrix destinations. [Profile tests](../crates/fc-api/tests/export_profiles.rs) cover native reopen, undo, malformed profiles and unique row destinations; matrix validates collisions before writes.                                                                                 |
| 59  | Retain, named formats                                   | [Export dispatch](../crates/fc-api/src/handlers/export.rs) routes the listed static/variable binary and UFO/designspace outputs. [Independent CLI checks](p5f-f-export-acceptance.md) decode and shape these outputs. Type 42 and bare CFF already have bounded implementations and user routes (correction below); general semantic TTX remains outside the implemented raw-table subset.                                                                                          |
| 60  | Retain                                                  | [Geometry export tests](../crates/fc-compile/tests/export_geometry.rs) and [CFF subroutine tests](../crates/fc-sfnt/tests/cff_subroutinize.rs) inspect emitted geometry/charstrings. Options reach the compiler through [compile_options](../crates/fc-api/src/handlers/export.rs); source-copy preprocessing preserves the editing document.                                                                                                                                       |
| 61  | Retain bounded bridge; require fresh real-tool evidence | [Hint handler](../crates/fc-api/src/handlers/hints.rs) reports availability/version and validates options; [external bridge](../crates/fc-hint/src/external.rs) handles process deadlines, invalid output and warnings. [Hint acceptance](p5f-k-hinting-acceptance.md) independently checks PS analysis and `cvar`, but explicitly records no real external binary run. Wave-3 owner must supply that result.                                                                       |
| 62  | Partial                                                 | [Release workflow](../.github/workflows/release.yml) contains native build/install jobs; [release acceptance](p5f-n-release-acceptance.md) has historical Linux extraction/runtime evidence. Workflow source does not establish macOS/Windows execution, and license notice generation alone does not establish AppImage redistribution readiness. Actual installer results remain necessary. Signing/publication are now deferred.                                                 |
| 63  | Retain                                                  | [CLI profile tests](../crates/fc-cli/tests/profiles.rs) compare byte-identical CLI/editor output, listing, all instances, variable subsets, preflight, missing profiles and conflicting flags. Shared [profile resolution](../crates/fc-api/src/handlers/export.rs) avoids a separate CLI option interpretation.                                                                                                                                                                    |
| 64  | Retain                                                  | [Bitmap apply/state](../crates/fc-api/src/handlers/bitmap.rs) names `(ppem, depth)` and checks generation; [document bitmap tests](../crates/fc-doc/tests/bitmap_commands.rs) verify pixel history, failed-batch rollback, rename/delete ownership and native reopening. Strike metadata edits correctly remain possible when only glyph pixels are locked.                                                                                                                         |
| 65  | Retain                                                  | [BitmapCanvas tests](../app/src/views/bitmap/BitmapCanvas.test.tsx) check toggle-then-match pencil behavior, eyedropper without writes, panning, move versus Alt-copy and one commit per drag. [Commands](../app/src/views/bitmap/bitmapCommands.ts) register tools in the shared shell. Zoom is a keyboard-accessible select, not a dedicated tool shortcut.                                                                                                                       |
| 66  | Retain                                                  | [BitmapCanvasControls](../app/src/views/bitmap/BitmapCanvasControls.tsx) connects independent grid, outline, advance-column and font-unit switches; readouts derive pixels and deltas from actual units-per-pixel. [Canvas tests](../app/src/views/bitmap/BitmapCanvas.test.tsx) check switch persistence and pixel/unit HUD behavior.                                                                                                                                              |
| 67  | Retain                                                  | [Rasterize handler](../crates/fc-api/src/handlers/bitmap.rs) snapshots source/generation, supports glyph selection or whole font, and applies validated output atomically. [Parity tests](../crates/fc-api/tests/bitmap_parity.rs) reject invalid threshold, depth and ppem without changes; [acceptance](p5f-g-bitmap-acceptance.md) independently renders stored results.                                                                                                         |
| 68  | Retain                                                  | [Trace handler](../crates/fc-api/src/handlers/bitmap.rs) maps all four modes and an independent diagonal-join option, batches replacements and widths into one history entry. [Parity tests](../crates/fc-api/tests/bitmap_parity.rs) check batching, missing images and geometry diagnostics; [acceptance](p5f-g-bitmap-acceptance.md) audits winding/closure across mode and join combinations.                                                                                   |
| 69  | Retain                                                  | [Bitmap parity tests](../crates/fc-api/tests/bitmap_parity.rs) edit the reusable pixel glyph and check both composite-preserving and decomposed export paths. [BitmapOutlineTools](../app/src/views/bitmap/BitmapOutlineTools.tsx) supplies the actual user action. Dependent geometry changes rather than freezing copies of the original pixel.                                                                                                                                   |
| 70  | Retain                                                  | [Grid handler](../crates/fc-api/src/handlers/bitmap.rs) enumerates exact divisors within supported ppem bounds and suggests the nearest exact size for a non-exact request. [Grid tests](../crates/fc-api/tests/bitmap_parity.rs) cover 1000/1024/2048 UPM families. The UI discloses non-integer choices rather than incorrectly claiming they are exact.                                                                                                                          |
| 71  | Review native PNG save                                  | [BDF tests](../crates/fc-formats/tests/bitmap_bdf.rs) cover properties and pixels; [BitmapSheet](../app/src/views/bitmap/BitmapSheet.tsx) validates selected names/layout, checks every glyph generation and commits one SetGlyphs batch. Embedded PNG layout is preferred on import. Native Download action has finding 1; browser download environment remains a separate current gap.                                                                                            |
| 72  | Retain                                                  | [ShapingCompile](../crates/fc-compile/src/shaping_compile.rs) compares all non-layout dependencies and glyph order before reusing outline/metric tables; layout-only updates replace GSUB/GPOS/GDEF/BASE. Other edits safely fall back to full compilation. [Feature acceptance](p5f-e-features-acceptance.md) verifies live shaping; “incremental” does not mean the first build skips font compilation.                                                                           |
| 73  | Retain                                                  | [PreviewStrip](../app/src/views/text/PreviewStrip.tsx) provides invert, flip, blur, sizes and detach; [usePreviewInstance](../app/src/views/text/usePreviewInstance.ts) requests interpolated geometry for named instances and carries shaping settings/direction. Shared persisted settings connect the detached window. [Proof acceptance](p5f-h-proofing-acceptance.md).                                                                                                         |
| 74  | Retain                                                  | [Waterfall parser](../app/src/views/text/waterfallSizes.ts) bounds sizes/counts and supports list/range increments; [PreviewStrip](../app/src/views/text/PreviewStrip.tsx) has separate single-glyph/run modes and apply-time errors. [Parser tests](../app/src/views/text/waterfallSizes.test.ts) and [wording acceptance](p5f-wording-translation-acceptance.md) preserve the size-versus-increment distinction.                                                                  |
| 75  | Retain                                                  | [Proof shaping tests](../crates/fc-api/tests/proof_shaping.rs) distinguish unspecified/default-on/default-off feature states. [VariableProofPanel](../app/src/views/text/VariableProofPanel.tsx) sends current features, direction, script, language and axis coordinates; [TextView](../app/src/views/text/TextView.tsx) exposes cluster inspection. Arbitrary axis proofs are a separate panel.                                                                                   |
| 76  | Retain                                                  | [ProofLibraryPanel](../app/src/views/text/ProofLibraryPanel.tsx) exposes grouped/script-aware samples and import/export. [Document tests](../crates/fc-doc/tests/proof_library.rs) pin undo and `.fcf` data; built-ins are adopted into project data through one command, rather than existing solely in browser storage.                                                                                                                                                           |
| 77  | Retain                                                  | [Paste dialog](../app/src/views/crossdocument/PasteSpecialDialog.tsx) routes destination and content choices to kernel paste; [cross-paste tests](../crates/fc-api/tests/cross_paste.rs) cover the 12 combinations, preservation of unselected fields, stale source/late failure, history and native reopen.                                                                                                                                                                        |
| 78  | Retain                                                  | [Cross-paste tests](../crates/fc-api/tests/cross_paste.rs) verify component relinking, missing-base policy, distinct same-name bases, order-independent mapping, cycles and non-exporting placeholder tags. [Automation acceptance](p5f-i-automation-acceptance.md) connects placeholders to the real inventory filter.                                                                                                                                                             |
| 79  | Retain typed API scope                                  | [Macro worker](../app/src/automation/macro.worker.ts) executes trusted JavaScript in a terminable worker; [automation tests](../crates/fc-api/tests/automation.rs) pin captured-document transactions, folder isolation and expiry. Generated declarations provide TypeScript API typing; direct TypeScript source must first be compiled, as [user docs](user/automation.md) say. This is not a bundled TypeScript compiler.                                                       |
| 80  | Retain                                                  | [Token tests](../app/src/styles/tokens.test.ts) check theme parity, text/marker contrast and separate bitmap/canvas colors. [Help acceptance](p5f-m-help-acceptance.md) verifies persisted language switching and English UI command. Catalogue equality is a useful structural check, not a claim that every visual/native surface has been tested.                                                                                                                                |
| 81  | Retain                                                  | [EditorController](../app/src/editor/editorController.ts) propagates hit distance and handle size into geometry; [editor acceptance](p5f-c-editor-acceptance.md) covers DPR rendering and CSS-pixel hit testing. Configurable marker size affects actual targets rather than only appearance.                                                                                                                                                                                       |
| 82  | Retain                                                  | [SmartComponentControls](../app/src/views/glyph/panels/SmartComponentControls.tsx) and [canvas pointer handling](../app/src/views/glyph/panels/smartComponentPointer.ts) edit local axis values. [Kernel tests](../crates/fc-api/tests/smart_component_editing.rs) verify independent instances, endpoints, invalid-value atomicity, history and native reopen; handles commit on release.                                                                                          |
| 83  | Retain bounded affine attachment                        | [Construction handler](../crates/fc-api/src/handlers/constructions.rs) validates sources, carrier geometry, cycles, locks and generations before one commit. [Construction tests](../crates/fc-api/tests/constructions.rs) cover preview/apply/bake/undo and source identity; [acceptance](p5f-j-construction-acceptance.md) verifies all four named kinds and exported geometry.                                                                                                   |
| 84  | Retain numeric scope                                    | [SmartCornerField](../app/src/views/glyph/SmartCornerField.tsx) exposes signed numeric radii; [document tests](../crates/fc-doc/tests/smart_corners.rs) retain radii through history and native reopening. [Independent acceptance](p5-smart-corners-acceptance.md) covers round/ink-trap output. Dedicated radius-drag handles and crossing construction remain outside the numeric claim.                                                                                         |
| 85  | Retain                                                  | [NodeLinksPanel](../app/src/views/glyph/panels/NodeLinksPanel.tsx) constructs linked/stem guides; [node-link command tests](../crates/fc-doc/tests/node_link_commands.rs) validate guide ownership/locks and propagation. [Construction acceptance](p5f-j-construction-acceptance.md) includes the actual magnet drag, not only inspector metadata. Stem detection is straight-stem detection.                                                                                      |
| 86  | Retain                                                  | [NodeLinksPanel](../app/src/views/glyph/panels/NodeLinksPanel.tsx) creates/removes independent X/Y servants from selection; [node-link tests](../crates/fc-doc/tests/node_link_commands.rs) exercise cycles, ownership, resolved movement and saved linkage. It is a live relation, not a one-time interpolated point copy.                                                                                                                                                         |
| 87  | Retain                                                  | [LayerEffectsEditor](../app/src/views/glyph/panels/LayerEffectsEditor.tsx) creates variable-stroke points/caps/joins and requires a reviewed generation before apply/bake. [Variable-pen tests](../crates/fc-doc/tests/variable_pen.rs) cover source recomputation, malformed profiles, history and native reopening; [independent stroker acceptance](p5f-j-construction-acceptance.md) measures TTF/OTF output.                                                                   |
| 88  | Retain stored-mask scope                                | [Effects handler](../crates/fc-api/src/handlers/effects.rs) captures evaluated paths, checks locked sources and stale generations, and applies ordered stacks. [Effects tests](../crates/fc-doc/tests/effects.rs) verify dependent invalidation, source preservation and failed-evaluation rollback. [Acceptance](p5-layer-effects-acceptance.md) explicitly defines copied masks rather than dynamic linked masks.                                                                 |
| 89  | Retain                                                  | [EditorController](../app/src/editor/editorController.ts) gives a refusal reason when a hit targets a locked path/glyph. [Command preflight](../crates/fc-doc/src/command/apply.rs) validates component and metadata locks centrally; [editor metadata tests](../crates/fc-doc/tests/editor_metadata.rs) establish persistence and refusal. Path focus is an editing selection/view mode.                                                                                           |
| 90  | Retain                                                  | [MeasureTool](../app/src/editor/tools/measure.ts) intersects actual paths, exposes per-span distances, italic-aware HUD and creates a guide on G during a drag. [Tool tests](../app/src/editor/tools/measure.test.ts) pin geometry and key handling. Persistent guide-crossing labels use a separate bounded polyline approximation.                                                                                                                                                |
| 91  | Retain                                                  | [AnnotationTool](../app/src/editor/tools/annotation.ts) captures layer ownership, rechecks locks and rereads annotations at release so concurrent entries are not overwritten. [Annotation tests](../app/src/editor/tools/annotation.test.ts) and [metadata tests](../crates/fc-doc/tests/editor_metadata.rs) cover gesture and stored layer data.                                                                                                                                  |
| 92  | Retain                                                  | [Image handler](../crates/fc-api/src/handlers/image.rs) resolves relative links, returns broken-link paths, checks generations/locks and allows repair of corrupt embedded images. [Image tests](../crates/fc-api/tests/image.rs) cover relinking, changed sources, transforms, source persistence and atomic refusal. [ImagePanel](../app/src/views/glyph/panels/ImagePanel.tsx) provides the user route.                                                                          |
| 93  | Retain threshold-vectorization scope                    | [Image trace](../crates/fc-api/src/handlers/image.rs) accepts threshold, curve fitting/tolerance, smoothing and minimum area; [image tests](../crates/fc-api/tests/image.rs) require curve nodes and enforce option limits. [Acceptance](p5f-j-construction-acceptance.md) measures deviation/winding and preserves straight corners. Photographic multitone tracing is not implemented.                                                                                            |
| 94  | Retain sampled optical scope                            | [Optical handler](../crates/fc-api/src/handlers/optical.rs) creates a preview and applies only accepted proposal rows against the captured generation. [Optical tests](../crates/fc-api/tests/optical.rs) cover component geometry, explicit zero/class rules, master scope, stale/empty reviews and native reopening. Sampling is disclosed; no exact optical solver claim.                                                                                                        |
| 95  | Retain                                                  | [proofStepping](../app/src/views/text/proofStepping.ts) permits bare Up/Down only, excludes IME/modifiers/interactive controls and yields to selected-node nudges. [Stepping tests](../app/src/views/text/proofStepping.test.ts) and [proof acceptance](p5f-h-proofing-acceptance.md) cover both field and canvas routes.                                                                                                                                                           |
| 96  | Retain bounded compare/merge                            | [Comparison tests](../crates/fc-api/tests/font_compare.rs) cover outlines, metadata, kerning, layout, components and all masters, plus kernel-enforced read-only references. [Merge tests](../crates/fc-api/tests/font_merge.rs) verify explicit decisions, one history entry and save/export. Merge excludes layout/bitmap/color/global-hint adoption; comparison still reports supported differences.                                                                             |
| 97  | Retain axis-position scope                              | [VirtualMastersEditor](../app/src/views/fontinfo/VirtualMastersEditor.tsx) stages named locations with the design-space transaction; [construction acceptance](p5f-j-construction-acceptance.md) checks sparse-source creation and independent `fvar`/`avar`/`gvar` instantiation. Virtual masters define positions; they do not imply a complete new master containing every glyph.                                                                                                |
| 98  | Retain documented PS/CFF scope                          | [HintPanel](../app/src/views/hints/HintPanel.tsx) reaches glyph/global PS editing; [hint handler](../crates/fc-api/src/handlers/hints.rs) validates generation and batches edits. [CFF hint tests](../crates/fc-compile/tests/hints.rs) cover private data, ghosts and replacement/counter masks. [Hint acceptance](p5f-k-hinting-acceptance.md) distinguishes Type 1 dotsection from CFF/CFF2 capabilities and flex binding limits.                                                |
| 99  | Retain nine supported forms                             | [VisualLookupEditor](../app/src/views/features/VisualLookupEditor.tsx) serializes forms into the persisted feature draft and protects automatic/read-only blocks. [Kernel tests](../crates/fc-api/tests/visual_lookup_authoring.rs) compile and shape each listed GSUB/GPOS type, class forms, invalid drafts and save/history. Arbitrary context graphs remain source-edited.                                                                                                      |
| 100 | Retain documented dialect                               | [Feature-token tests](../crates/fc-compile/tests/feature_tokens.rs) demonstrate actual shaping changes, static/variable branches and original Unicode source offsets for errors. [Feature acceptance](p5f-e-features-acceptance.md) identifies the accepted number/property/predicate grammar; there is no claim to execute arbitrary preprocessor code.                                                                                                                            |
| 101 | Retain horizontal strike-proof scope                    | [BitmapWaterfall](../app/src/views/bitmap/BitmapWaterfall.tsx) requests each strike and names unencoded selected glyphs directly. [Parity tests](../crates/fc-api/tests/bitmap_parity.rs) check cross-strike filters, metrics/origin scaling and undo; [waterfall tests](../app/src/views/bitmap/BitmapWaterfall.test.tsx) cover unencoded glyphs. Vertical coverage is the row 111 finding.                                                                                        |
| 102 | Retain                                                  | [Bitmap table writer](../crates/fc-sfnt/src/bitmap/write.rs) implements the EBDT/EBLC, CBDT/CBLC and sbix paths. [Bitmap acceptance](p5f-g-bitmap-acceptance.md) independently decodes payloads and renders color strikes with PNG-enabled FreeType. Whole-font color export and color-table preference when mono/color coexist remain explicit boundaries.                                                                                                                         |
| 103 | Retain conservative detection                           | [BitmapReconstruction](../app/src/views/bitmap/BitmapReconstruction.tsx) auto-detects on document/master changes and rechecks immediately before reconstruction. [Pixel reconstruction tests](../crates/fc-raster/tests/pixel_reconstruction.rs) reject off-grid/curved false positives and bound expanded component work; [API parity tests](../crates/fc-api/tests/bitmap_parity.rs) cover TTF/OTF round trips and normal outline fonts.                                          |
| 104 | Retain static color minimum                             | [ColorEditor](../app/src/views/color/ColorEditor.tsx) exposes palette/layer authoring; [color render tests](../crates/fc-api/tests/color_render_state.rs) preserve imported ownership/raw bytes and selected-master geometry. [Rendering acceptance](p5-color-rendering-acceptance.md) supplies actual previews. Variable paints and color proof exports remain outside this static v0-minimum claim.                                                                               |
| 105 | Retain local/bundled scope                              | [Automation tests](../crates/fc-api/tests/automation.rs) exercise a separate Python interpreter, RPC deadlines/reaping, captured-document restrictions and output limits. [Plugin registry](../app/src/plugins/registry.ts) accepts only bundled implementations matching curated provenance/version; [manager tests](../app/src/plugins/PluginManager.test.tsx) cover lifecycle/settings. It is not a third-party downloader.                                                      |
| 106 | Retain                                                  | [drawPluginOverlays](../app/src/plugins/drawPluginOverlays.ts) receives document/view context; [overlay tests](../app/src/plugins/drawPluginOverlays.test.ts) check failure isolation. Filter plans use generation-checked style preview/commit with cancel and stale rejection. [Automation acceptance](p5f-i-automation-acceptance.md) discloses suppression in shaped-text canvas mode and no reporter pixel-diff run.                                                           |
| 107 | Retain                                                  | [QuickHelp](../app/src/shell/QuickHelp.tsx) resolves live bindings and opens the contextual article. [Quick Help tests](../app/src/shell/quickHelp.test.tsx) exercise hover/focus/F1/modal boundaries and keyboard entry; [command help tests](../app/src/shell/commandHelp.test.ts) require every registered command to have translated help. The full article is modal, not a persistent dock.                                                                                    |
| 108 | Retain preset subsets                                   | [Preset definitions](../app/src/shell/keymapPresets.ts) provide the named importable subsets; [integration test](../app/src/shell/keymapIntegration.test.ts) checks merged default/preset conflicts. [Help acceptance](p5f-m-help-acceptance.md) verifies persisted overrides, JSON import/export and actual changed-binding dispatch. The presets are not exhaustive replicas of competitors.                                                                                      |
| 109 | Retain export implementation; refresh download evidence | [ProofExportControls](../app/src/views/text/ProofExportControls.tsx) shapes and embeds the font inside one settled-document barrier, checks generation, and saves through the native/browser helper. [prepareProofSheet](../app/src/views/text/prepareProofSheet.ts) rejects unresolved/stale runs; [proof acceptance](p5f-h-proofing-acceptance.md) decodes PDF/HTML independently. Current bundled-browser download cancellation needs fresh evidence.                            |
| 110 | Retain explicit CID scope                               | [CID handler](../crates/fc-api/src/handlers/cid.rs) checks generation and batches metadata plus numeric order. [CID export tests](../crates/fc-compile/tests/cid.rs) reject incomplete/duplicate mappings and decode non-identity CIDs, FDArray and FDSelect. [CidEditor](../app/src/views/fontinfo/CidEditor.tsx) reports unmapped exported glyphs rather than all non-exporting inventory entries.                                                                                |
| 111 | Partial literal scope; decision required                | [Vertical tests](../crates/fc-doc/tests/vertical.rs) and [VORG export tests](../crates/fc-compile/tests/otf_vertical.rs) verify edited/inherited metrics and minimal override records. Text/variable proof pipelines propagate `ttb`, and [proof acceptance](p5f-h-proofing-acceptance.md) verifies vertical PDF/HTML. The bitmap waterfall remains horizontal: finding 2. Edit View vertical mode promotes metrics; it does not vertically arrange neighboring edit glyphs.        |
| 112 | Partial until stale-grid fix                            | [CJK resolver](../app/src/views/glyph/panels/cjkResolve.ts) rejects ambiguous/missing/self/empty radicals and normalizes source widths. [Document tests](../crates/fc-doc/tests/cjk_construction.rs) cover atomic IDS construction, cycles, component locks, transformations and native reopen. [CjkTransformGrid](../app/src/views/glyph/panels/CjkTransformGrid.tsx) stages one batch, but the delayed-Apply regression reproduces finding 3. No bundled IDS database is claimed. |

## Verification and limits of this review

- Read the shared protocol, current feature/resume audit, STATUS/TODO and the
  relevant acceptance/review sections. Inspected the implementation and named
  regression assertions above, including user entry points and kernel validation.
- No product implementation was changed. No Cargo/Playwright or broad Vitest
  suite was run. After this read-only row review, root authorized one focused
  Vitest regression for finding 3; its exact failing result appears below. All
  other cited test evidence remains historical.
- A small Python structural check validates row coverage, referenced local files
  and English/Chinese leaf-key equality. Final output is recorded below.
- No dependency, new translation or FontForge adaptation was introduced. The
  audit changed this report, its task file and the authorized CJK test only. Test
  ownership was handed to `w3_cjk_editing` immediately after reproduction.

```text
PASS: rows 57–112 exactly once; 152 local links resolve
Catalogue leaves: en=3936, zh-CN=3935, en-only=1, zh-only=0
New in-progress English key: shortcuts.fixedSpacing

python3 scripts/check_artifacts.py
build: 15.59 GiB, 19,162 files
temporary: 0.32 GiB, 2,062 files
workspace free: 606.07 GiB
warnings: build artifacts exceed 10 GiB; plan profile-cache cleanup
PASS
```

The catalogue difference belongs to concurrent wave-3 keyboard work and needs
the separately assigned translation pass before integration. It is not a failure
of the previously completed wording pass.

## Parent-authorized reproduction: row 112

Added `refuses a captured component draft if the list changes while settling`
to [CjkTransformGrid.test.tsx](../app/src/views/glyph/panels/CjkTransformGrid.test.tsx).
The test stages X = 420 on the `uni6728` component, suspends `settle`, clicks Apply,
replaces the component with `uni53E3` through a rerender and then releases the
barrier. Existing source still sends a SetComponent for `uni6728` at index 0.

```text
TMPDIR=$PWD/.scratch/p5f-w3-audit-b/tmp pnpm --dir app exec vitest run \
  --maxWorkers=1 --minWorkers=1 src/views/glyph/panels/CjkTransformGrid.test.tsx

FAIL refuses a captured component draft if the list changes while settling
Expected: []
Received: doc.apply Batch[SetComponent { index: 0, component: {
  base: "uni6728", transform: [0.6, 0.1, -0.2, 0.8, 420, -15], ...
}}]
Test Files 1 failed (1)
Tests 1 failed | 9 passed (10)
Duration 3.33s
```

The failing test is intentional evidence, not a passing integration check. The
CJK fix owner received its exact output and exclusive test-file ownership; root
will integrate that owner's fixed result. The task's test process exited and
its own empty temporary directory was removed.

## Correction to the formats backlog: Type 42 and bare CFF exist

The old “Type 42 and bare CFF remain design-required and unbuilt” statement in
TODO is inaccurate. The individual [Type 42 handoff](p5-type42-acceptance.md) and
[bare CFF handoff](superpowers/plans/tasks/p5-42-bare-cff-report.md) predate their
registration and explicitly leave execution to integration. Current source and
subsequent integration logs supersede that unbuilt/standalone-only description.

- **Open/import API:** [session.open](../crates/fc-api/src/handlers/session.rs)
  routes `.cff` to `cff_interchange::read_path`; `.t42`, `.p42`, `.type42` are
  decoded through the Type 42 wrapper reader and then the existing sfnt importer.
  Imported warnings and source paths are retained. Bare CFF rejects a nonzero
  face selection because it accepts one font only.
- **GUI export:** [exportFormats](../app/src/dialogs/exportFormats.ts) puts `Cff`
  and `Type42` in the ordinary selector, supplies `.cff`/`.t42` output extensions,
  and uses existing translated labels. [ExportDialog](../app/src/dialogs/ExportDialog.tsx)
  disables variable output for these formats. The shared
  [export handler](../crates/fc-api/src/handlers/export.rs) compiles static OTF
  then extracts `CFF ` with an explicit container-loss warning, or compiles
  TrueType then wraps it in Type 42. The destinations are written only after
  successful conversion. Static CFF subroutinization is supported.
- **CLI:** [convert](../crates/fc-cli/src/cmd/convert.rs) and
  [build](../crates/fc-cli/src/cmd/build.rs) map CFF/Type 42 formats. The shared
  [CLI reader](../crates/fc-cli/src/cmd/mod.rs) opens these extensions through
  their codecs. [Interchange CLI tests](../crates/fc-cli/tests/interchange.rs)
  actually convert a bounded FCF fixture into `.cff` and `.t42` and inspect both
  outputs through that reader, rather than merely checking enum registration.
- **Existing executed evidence:** [integration reverification log](acceptance/p5-resume-reverification.log)
  records Type 42 **8 passed** and bare CFF **4 passed**. The later
  [CLI repair run](acceptance/p5-resume-regression.log) records both interchange
  CLI tests **2 passed**, including `new_interchange_outputs_reopen_through_cli_reader`.
  The earlier per-codec reports' interrupted/not-run notes must stay historical;
  they do not invalidate these later execution records. Final integrated tests
  should repeat their normal coverage as part of the existing workspace gate.

**Actual remaining ordinary-workflow gap:**
[OPEN_FONT_EXTENSIONS](../app/src/shell/nativeDialogs.ts), consumed by
[sessionStore.openDialog](../app/src/stores/sessionStore.ts), currently omits
`cff`, `t42`, `p42`, `type42` and also `svg`, `eps`, `afm`, `pfm`, `ttx`, despite
API support. The native picker sends only that named filter, so those source
formats are not normally discoverable there. Browser path entry, recent-file
reopen, API and CLI paths are separate routes. Add all supported extensions and
a registry/picker regression; no new codec is needed. This mismatch was sent to
root, which owns the corrective task.

**Bounded codec coverage that must remain visible:**
[bare CFF](../crates/fc-sfnt/src/cff_interchange.rs) accepts single-font CFF1 up
to 128 MiB, retains supported outlines/advances/hints/CID identity and infers
Unicode only from unambiguous names. CFF2 and multi-font CFF sets are refused;
there is no standalone Unicode cmap or layout table to preserve.
[Type 42](../crates/fc-formats/src/type42/mod.rs) accepts a bounded declarative
hexadecimal wrapper without running PostScript. It extracts original sfnt data;
wrapper aliases/reencoding do not rewrite cmap/post. Binary/computed sfnts,
executable glyphs, unsupported transforms/metric overrides/CID wrappers are
not supported. The writer refuses indivisible tables/glyphs over 65,534 bytes.
These are implemented constrained interchange formats, not unrestricted
PostScript or lossless editable source containers. [User documentation](interchange-formats.md)
already describes their working paths and principal limits.

## Native Open filter correction handoff

Root separately authorized the narrow filter fix after this independent audit.
It is now implemented and independently accepted by `w3_review_release`:
[filter acceptance](p5f-w3-open-filter-acceptance.md). The finding above preserves
what the audit observed; the corrective report records the resulting 13/13
boundary-test result. Codec scope and remaining native-platform execution gaps
are unchanged. The CJK and bitmap findings remain owned by their separate fix
agents and require their final reports for closure.
