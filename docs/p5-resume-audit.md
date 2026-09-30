# P5 resume evidence reconciliation

Date: 2026-09-30. Historical implementation baseline: `91db6fa`.

The user's later instruction in this resume session freezes new work: finish the
currently active integrations, then stop and rest. The unchecked acceptance
units below document remaining scope; they are not authorization to start another
implementation batch. The integration owner records the final stopping point.

## Review scope and method

This bounded documentation task reconciles the 112 existing feature-audit rows
against P0–P5 acceptance records and representative code/tests, then splits the
largest remaining product areas into explicit acceptance gates. It changes only
this report and `progress-estimate.md`; implementation, the authoritative audit,
STATUS and TODO belong to the integration owner.

Three independent read-only reviewers cover rows 1–37/97–103,
38–72/91–96 and 73–90/104–112. The shared checkout is concurrently receiving
new implementation. New files and in-progress changes are not acceptance
evidence. Historical passing tests retain their recorded scope; this review
does not claim a fresh application build, regression run or browser session.

The initial resource check passed: build artifacts 5.23 GiB / 12,405 files,
temporary artifacts 0.00 GiB / 3 files and 670.49 GiB workspace free.

No pending row becomes accepted merely because a source file or helper exists.
This report is a historical-evidence reconciliation, not the acceptance report
for the parallel implementation now in progress.

## Findings and update recommendations

The source audit at resume contained **3 Complete, 3 Partial and 106 Pending
verification** rows. Of those pending rows, **13 have strong reusable acceptance
evidence for the stated requirement**: **3, 21, 22, 25, 28, 29, 45, 51, 59, 64,
67, 102 and 110**. Their underlying tests and browser/interoperability records
already passed in earlier milestones. The integration owner should attach those
records when deciding their authoritative status; they are recommendations here,
not thirteen newly run acceptance tests.

Rows **84, 88 and 104** retain their existing accepted scopes. The other **96**
rows comprise **76 with reusable subsets** and **20 without an accepted
end-to-end workflow**. They require a missing qualifier, broader verification, or
acceptance of a workflow absent from the historical evidence. This does not mean 96
features must be built from scratch. Most have an implemented, accepted subset.
Conversely, neither 16/112 nor a phase/task count measures product effort.

The most consequential distinctions are:

- Native FCF is a text JSON document with per-glyph lines; this does not satisfy
  row 7's per-glyph-file source tree. Recovery exists, but uses an application
  autosave directory rather than an adjacent source sidecar.
- The accepted FEA source workflow and the typed binary lookup inventory are
  separate paths. Their existence does not prove a complete FEA-to-typed-IR
  round trip (46) or editable visual lookup authoring (99).
- P4 interpolation is an adjacent read-only preview. Numeric smart-component
  parameters do not establish canvas axis scrubbing, smart handles or the full
  compatibility repair interaction (55, 57, 82).
- The required core formats in row 59 already have acceptance. The approved
  design's wider interchange table also includes **Type 42 and bare CFF**;
  these must remain explicit work alongside SVG/EPS/AFM/PFM/TTX/collections.
- Linux installation was actually tested. A Linux build or platform CI
  definition cannot close macOS/Windows installation or native parity (9, 62).
- Historical P3/P4/hint reports describe a Tibetan layout-budget failure that
  was fixed before this resume. The later STATUS/P5-plan correction supersedes
  that old failure note; it does not retroactively turn the older excluded runs
  into unrestricted passes.

## Evidence key

| Key     | Recorded acceptance                                                                                                                                                                                                 |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0      | [Foundation](p0-acceptance.md): formats, native persistence, grid/performance, shell and Linux build.                                                                                                               |
| P1      | [Outline editing](p1-acceptance.md): editing/history, tools/panels, text, native recovery and exports.                                                                                                              |
| T6      | [P1 selection/editor report](archive/ledgers/2026-09-03-p1-outline-editing/task-6-report.md): exact interaction mapping, tests and isolated browser actions; final P1 acceptance incorporates its fix/review waves. |
| P2a     | [Kerning](p2a-acceptance.md): editable groups, pair scopes, transactional text-tab authoring and preservation.                                                                                                      |
| P2b     | [Layout](p2b-acceptance.md): FEA drafts, diagnostics, generators, source assembly and bounded binary conversion.                                                                                                    |
| P2cd    | [CFF/source formats/problems](p2cd-acceptance.md): static exports and explicit format/preservation limits.                                                                                                          |
| P3      | [Bitmap fonts](p3-acceptance.md): editor/strike persistence, raster/trace, legacy and embedded codecs, browser and independent decoding.                                                                            |
| P4      | [Variable/CJK](p4-acceptance.md): authoring/export, vertical metrics, CID, Linux installation and explicit preview/source limitations.                                                                              |
| Hint    | [Hinting checkpoint](p5-hinting-checkpoint.md): static TT/PS editing, native/binary export and independent rendering within its stated boundaries.                                                                  |
| Effects | [Layer effects](p5-layer-effects-acceptance.md): uniform pens and stored-copy boolean masks.                                                                                                                        |
| Corners | [Smart corners](p5-smart-corners-acceptance.md): numeric signed-radius editing, native persistence, bake and independent geometry exports.                                                                          |
| Color   | [COLR/CPAL](p5-color-acceptance.md): static v0 palettes/layers, history, native persistence and binary outputs.                                                                                                     |

Representative implementation checks included `fc-doc/src/fcf`,
`MetricsPanel`, `LookupInspector`, `FeatureSourceTree`, `VariationPreview`,
`SourceLayerEditor`, `ComponentRow`, `PreviewStrip`, `Viewport`, `hitTest`,
`InfoMultiRows`, `NumberField`, `hud`, the feature generator registry and the
shared text-shaping compiler. Tests checked for matching behavioral scope include
`App.test.tsx`, `EditInspector.test.tsx`, `useToolShortcuts.test.tsx`,
`InfoBox.test.tsx` and the proportional-nudge cases in `drag.test.ts`.
Concurrent edits to these entry points need their owners' later evidence.

## Row-by-row reconciliation

**A** retains an already accepted row. **R** recommends reusing prior acceptance
for the stated row. **P** has a verified subset but requires the listed qualifier
or direct verification. **N** has no accepted end-to-end workflow for the stated
requirement; related foundations may exist. These are review classifications,
not edits to the authoritative feature-audit statuses.

### Shell, inventory and outline editing

| ID  | Class | Reusable evidence and remaining qualifier                                                                                                    |
| --- | ----- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | P     | P0/P1 tabs and cycling; per-document windows, pinned Overview and prescribed ordinal bindings lack historical acceptance.                    |
| 2   | N     | No accepted OS-window tear-off workflow; new window work requires its own evidence.                                                          |
| 3   | R     | P1 contextual left sidebar and scrolling right Inspector sections; App/EditInspector tests cover active context and section order/state.     |
| 4   | P     | P1 collapsible bottom preview; draggable divider remains unverified.                                                                         |
| 5   | P     | P0/P1 command-search palette; filter/script/glyph providers and the prescribed Mod+K binding remain outside that evidence.                   |
| 6   | N     | Global UI preferences persist; named workspaces restored per document lack prior acceptance.                                                 |
| 7   | P     | P0/P1 FCF semantic round trip and crash recovery; per-glyph files and source-adjacent recovery sidecars are not the existing storage layout. |
| 8   | P     | P1 document history, labels and coalesced gestures; user-visible per-entry scope labeling remains to verify.                                 |
| 9   | P     | P1 key abstraction and Linux evidence; actual macOS/Windows/native parity remains unverified.                                                |
| 10  | P     | P0 65,535-glyph virtual grid/performance and size controls; GPU wording, list mode and rearrangeable columns lack prior acceptance.          |
| 11  | P     | Existing non-export/component indicators; all six requested badge semantics require acceptance.                                              |
| 12  | P     | Twelve glyph labels and metadata/filter tests; separate layer colors and tag autocomplete remain.                                            |
| 13  | P     | P0 name/Unicode/character search; notes, case control and regex need acceptance.                                                             |
| 14  | P     | P0/P4 category/script/block/Unihan filters; all subcategory/language/list counts and generate-missing workflow remain.                       |
| 15  | P     | Existing filter predicates; full project-persisted DSL and matching visual builder lack acceptance.                                          |
| 16  | P     | P0/P4 naming/classification/Unihan data; user overlays, anchor/composition data and Update Glyph Info remain.                                |
| 17  | P     | P1/P4 character/range/recipe additions; batch find-and-replace rename remains.                                                               |
| 18  | P     | P0/P4 CJK grid/radical/stroke/block filtering; empty cells lack accepted system-face fallback rendering.                                     |
| 19  | P     | P4 vertical advance/origin editing; explicit TSB/BSB Inspector controls require acceptance.                                                  |
| 20  | P     | P1 outline tools and P3 bitmap tools; complete Lasso/Eraser/Annotation/Pixel tool interaction and integration remain.                        |
| 21  | R     | P1/T6 tap versus 200 ms borrowed-tool behavior; unit tests and actual H borrow/release browser action.                                       |
| 22  | R     | P1/T6 double-click/Return smooth toggle and Alt preserving geometry; controller/document command tests.                                      |
| 23  | N     | G2 nodes and one-shot Harmonize have no prior accepted workflow.                                                                             |
| 24  | P     | P1/T6 constraints, 1/10/100 nudges and alternate gestures; exact Ctrl snap-suspension grammar/platform combinations need verification.       |
| 25  | R     | P1 proportional node-drag behavior and neighboring-handle scaling; proportional-nudge regression cases.                                      |
| 26  | P     | P1 segment insertion and Add Extremes; Force Extremes remains outside historical acceptance.                                                 |
| 27  | P     | P1 direction/reverse/tidy commands; all-master variants, Open Corner and reconnect workflow remain.                                          |
| 28  | R     | P1/T6 X/Y/W/H, nine-point origin, proportional lock and numeric keyboard fields; InfoBox tests and browser numeric/origin interaction.       |
| 29  | R     | P1/T6 pointer-following delta, angle, length and two-node stem readouts, with recorded browser observations.                                 |
| 30  | P     | P1 local/master named/angled/locked guides and measuring; LSB/RSB anchoring needs acceptance.                                                |
| 31  | P     | P1 node/handle/component/metric/guide/grid snapping; all independent target switches, zones/background and configurable distances remain.    |
| 32  | P     | P1 per-master background editing/copy/swap/clear; prescribed shortcut and selection-only transfer need verification.                         |
| 33  | P     | P1 automatic component alignment and decomposition; component locking remains unaccepted.                                                    |
| 34  | P     | P1 named anchors and mark conventions; mark-cloud preview remains.                                                                           |
| 35  | P     | P2cd explicit read-only problem report; always-on inline markers, hover details, fixes and per-test switches remain.                         |
| 36  | P     | P1/P4 master/background/backup/source layers; full arbitrary-layer add/duplicate/rename interaction needs acceptance.                        |
| 37  | P     | P1 reference/offset/mirror/literal metrics keys; local `==`, height `@` and auto-aligned-relative grammar remain.                            |

### Spacing, layout, export and bitmap editing

| ID  | Class | Reusable evidence and remaining qualifier                                                                                                                     |
| --- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 38  | P     | Current-layer out-of-sync/apply behavior; update across all masters needs acceptance.                                                                         |
| 39  | P     | P1/P2a LSB/RSB shortcuts; both sides, true-1 and cross-platform modifier behavior remain.                                                                     |
| 40  | P     | P2a kerning enable/disable; spacing-protecting locked mode remains.                                                                                           |
| 41  | P     | P2a groups and explicit pair scopes; modifier-held class suppression remains.                                                                                 |
| 42  | P     | P2a editable gap controls; positive/negative color distinction and group ghosts remain.                                                                       |
| 43  | P     | P2a group CRUD; complete sortable/filterable table, scoped rename, cleanup/compress and cross-master clipboard remain.                                        |
| 44  | P     | P1/P2a text spacing; editable matrix, beam/cutoff display and saved proof strings remain.                                                                     |
| 45  | R     | P4 vertical metrics and model/manual/group vertical kerning, browser editing and compiled output.                                                             |
| 46  | P     | P2b FEA source assembly and typed binary lookup conversion; complete typed source-IR parse/serialize round trip is not established.                           |
| 47  | P     | P2b prefix/class/feature tree and generated-source protection; arbitrary per-entry automatic/manual authoring remains.                                        |
| 48  | P     | P2b seven generator families; the entire enumerated suffix/number/fraction/ordinal/superior/inferior/aalt recipe set requires verification or implementation. |
| 49  | P     | P2a/P2b model-generated kerning/anchor layout; directly verify all four requested tags and their source/export precedence together.                           |
| 50  | P     | P2b located diagnostics and completion; Alt-click class preview remains.                                                                                      |
| 51  | R     | P2b/P4 accepted `vert`/`vrt2` generation, alternate handling and vertical compile/browser workflows.                                                          |
| 52  | P     | P4 master overrides; complete Family→Font→Master→Export inheritance and indicators remain.                                                                    |
| 53  | P     | P1/P4/Hint metrics, zones, stems and grid; full editable UPM/subdivision/increment behavior remains to verify.                                                |
| 54  | P     | P4 axes/masters/instances and source-subset export; persistent variable export configuration remains.                                                         |
| 55  | P     | P4 adjacent preview and topology repair; shape indices/colors, connectors, status segments and drag-to-swap remain.                                           |
| 56  | P     | P4 brace coordinates and single-axis bracket bounds; the explicit compound-condition boundary remains open.                                                   |
| 57  | P     | P4 adjacent interpolation preview; live scrubbing of the Edit View drawing remains.                                                                           |
| 58  | P     | Existing export options and problem navigation; stored profiles, instance/profile matrix and full preflight workflow remain.                                  |
| 59  | R     | P1/P2cd/P4 accepted TTF/CFF OTF/WOFF/WOFF2/variable TTF/UFO+designspace paths, within recorded format limits.                                                 |
| 60  | P     | P2cd/Hint overlap, production naming, rounding/decomposition paths; explicit extrema insertion and subroutinization need acceptance.                          |
| 61  | P     | Retain Hint's static PS analysis/external TT acceptance; variable hinting and wider analysis quality remain.                                                  |
| 62  | P     | P4 actual temporary Linux install and rendering; macOS/Windows installs remain unverified.                                                                    |
| 63  | P     | Shared API/CLI compiler and options; common persisted profiles and CI use remain.                                                                             |
| 64  | R     | P3 first-class `(ppem, depth)` strikes, per-strike metrics, history and native persistence.                                                                   |
| 65  | P     | P3 major drawing/selection tools; exact toggle-then-match pencil, eyedropper and hand interactions need acceptance.                                           |
| 66  | P     | P3 grid/baseline view; outline overlay, advance column, pointer/delta HUD and units-per-pixel remain.                                                         |
| 67  | R     | P3 selected/all-glyph rasterization with configurable monochrome threshold and undoable regeneration.                                                         |
| 68  | P     | P3 exact closed/simplified pixel boundaries; chamfer/round and diagonal-join choices remain.                                                                  |
| 69  | N     | No accepted tracing mode driven by an editable `pixel` component glyph.                                                                                       |
| 70  | N     | No accepted integer-unit grid preset workflow across the requested UPM families.                                                                              |
| 71  | P     | P3 BDF and typed property editing; PNG sheet interchange remains.                                                                                             |
| 72  | P     | P1/P2 live HarfRust shaping; current accepted compilation rebuilds a full font, not the requested layout-only incremental shaper.                             |

### Proofing, construction, automation and CJK completion

| ID  | Class | Reusable evidence and remaining qualifier                                                                                                                         |
| --- | ----- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 73  | P     | P1 preview/inversion; detachable preview, instance choice, flip and blur need acceptance.                                                                         |
| 74  | P     | P1 fixed-size text waterfall/P3 strike waterfall; configurable steps and single-glyph mode remain.                                                                |
| 75  | P     | Existing Boolean feature controls; tri-state, script/language overrides, axes and cluster inspection need full acceptance.                                        |
| 76  | P     | Built-in samples; grouped script-aware project library remains.                                                                                                   |
| 77  | N     | No prior accepted Paste Special target/payload matrix.                                                                                                            |
| 78  | N     | No prior accepted cross-document component relinking and missing-base placeholder workflow.                                                                       |
| 79  | P     | Generated typed API/transports; executable scripts, console, folder discovery and reload lack prior acceptance.                                                   |
| 80  | P     | P0/P1/P2a accepted bilingual themes and distinct canvas palettes; current added strings need the independent final localization barrier.                          |
| 81  | P     | P1 DPR-aware rendering and logical-pixel hit testing; configurable handle size remains.                                                                           |
| 82  | P     | P4 smart grids/numeric per-use parameters; actual axis sliders and smart handles remain.                                                                          |
| 83  | N     | No accepted corner/cap/segment/brush component attachment workflow.                                                                                               |
| 84  | A     | Corners: numeric live rounding/traps, history/native persistence and independent exports. Radius handles/crossing construction remain explicit supplemental work. |
| 85  | P     | P1 ordinary guide editing; node links, magnet workflow and automatic stem detection remain.                                                                       |
| 86  | N     | No accepted X/Y-interpolated servant-node workflow.                                                                                                               |
| 87  | P     | Effects: uniform ellipse/rectangle pens and caps/joins; variable positions/dimensions/angles/progression need independent acceptance.                             |
| 88  | A     | Effects: ordered stored-copy masks, source preservation, bake/undo/native persistence and independent exports; no dynamic-mask-link claim.                        |
| 89  | N     | Path focus and path/glyph locking have no complete prior workflow acceptance.                                                                                     |
| 90  | P     | P1 rulers/intersection measurement; italic-aware deltas and dropping a guide during measurement remain.                                                           |
| 91  | N     | No prior accepted per-layer text/arrow/circle/plus/minus annotation workflow.                                                                                     |
| 92  | N     | Image data fields alone do not establish relative-path import/transform/lock/broken-link behavior.                                                                |
| 93  | P     | P3 pixel-edge tracing; general raster-image autotrace and curve behavior remain.                                                                                  |
| 94  | P     | P4 batch metrics/centering; optical-density spacing and automatic kerning remain.                                                                                 |
| 95  | P     | Sample text exists; saved pair/phrase library with spacing-step navigation remains.                                                                               |
| 96  | N     | No prior accepted Compare Fonts/Family/reference-binary workflow.                                                                                                 |
| 97  | N     | No prior accepted virtual-master workflow.                                                                                                                        |
| 98  | P     | Retain Hint's exact PS/Type 1 subset; flex, dot sections, general counter control, seac inheritance and editable per-FD dictionaries remain.                      |
| 99  | P     | P2b read-only lookup inventory and source conversion; visual GSUB 1/2/3/4/6 and GPOS 1/2/4/6 editing remains.                                                     |
| 100 | N     | No prior accepted property/predicate/number tokens or `#ifdef VARIABLE` preprocessing workflow.                                                                   |
| 101 | P     | P3 waterfall, same-depth clipboard and scaling; explicit combined cross-strike copy/scale acceptance remains.                                                     |
| 102 | R     | P3 EBDT/EBLC, CBDT/CBLC and sbix editable embedding, independently decoded with FontTools; retain PNG/metrics format limits.                                      |
| 103 | P     | P3 decoded embedded strikes and explicit rasterization; pixel-outline auto-detection/reconstruction remains.                                                      |
| 104 | A     | Color: static COLR v0/CPAL authoring/history/native persistence and independent output checks; grid/text remain monochrome in this evidence.                      |
| 105 | N     | No prior accepted out-of-process Python RPC plus curated plugin-manager workflow.                                                                                 |
| 106 | N     | No prior accepted reporter drawing/filter-preview plugin contracts.                                                                                               |
| 107 | N     | Existing shortcut listing does not establish hover/F1/context-sensitive help.                                                                                     |
| 108 | N     | Existing fixed shortcuts do not establish imported presets and conflict-checked editing.                                                                          |
| 109 | N     | No prior accepted PDF/HTML proof export.                                                                                                                          |
| 110 | R     | P4 CID editor/order/native persistence and independently decoded CID-keyed CFF export.                                                                            |
| 111 | P     | P4 vertical Edit View and VORG/origin export; every proofing surface still needs vertical acceptance.                                                             |
| 112 | P     | P4 ordinary/smart component infrastructure; radical library, IDS input and component transform grid remain.                                                       |

## Bounded acceptance checklist for the largest remaining areas

Each item below is an acceptance unit, not a claim that current parallel code is
complete. For every editing unit, demonstrate the user entry point, deterministic
preview/apply behavior, invalid/stale-input rollback, one logical undo/redo unit,
FCF save/reopen and export where applicable. Use semantic or independent output
checks; build success alone is insufficient. UI units also need keyboard access,
English/Chinese review and isolated browser evidence. Record unsupported cases
without treating them as implemented.

### Geometry, construction and images

- [ ] **Variable pens (87):** insert/move/delete pen points; position, width,
      height, angle and progression; caps/joins; shared preview/compile evaluation;
      source persistence, bake/undo and independent variable-width output checks.
- [ ] **Attached construction (83, 85, 86):** separately accept each named
      corner/cap/segment/brush attachment, linked guide/magnet/stem detection, and
      X/Y servant interpolation; demonstrate dependency updates and invalid/cyclic
      reference handling rather than only storing new model fields.
- [ ] **Editor geometry (23–35, 81, 89–91):** preserve the existing interaction
      contract while accepting G2/harmonize, path variants, locking, annotations,
      configurable snapping/handles and measurement modifiers individually.
- [ ] **Images/tracing (92, 93):** relative-path import and reopen, transform/lock,
      missing-source indication, threshold/trace options, holes/winding and undo;
      disclose curve or photographic-trace limitations with representative images.
- [ ] **Optical metrics (94):** global density controls with captured-glyph
      preview, robust whitespace/component behavior, kerning proposals and explicit
      application; test real Latin/CJK examples and source/master scope.

### Documents, cross-document operations and family comparison

- [ ] **Paste matrix (77, 78):** test same-name/selected/component targets
      against all-data/layer/groups/metrics-key payloads; name/ID relinking,
      missing-base placeholders, cycles, selection mismatches and atomic failure.
- [ ] **Compare/reference (96):** select two documents or a reference binary;
      compare inventory, metadata, metrics, outlines, components and layout within
      a documented scope; make differences navigable and preserve both inputs.
- [ ] **Family comparison/merge:** compare compatible and mismatched master
      sets, resolve glyph/component/name conflicts explicitly, choose retained
      metadata/layout and verify one transaction plus export/reopen. A two-font
      diff or basic clipboard implementation does not close family merging.

### Automation and plugins

- [ ] **TypeScript scripts (79):** generated declarations usable by actual
      scripts; console result/error output; folder discovery/reload; document
      access, edit transaction/history and cancellation/failure behavior.
- [ ] **Python RPC (105):** launch a separate interpreter, version the protocol,
      transfer typed requests/errors, bound/reap failed jobs and preserve the
      document when a script fails before commit.
- [ ] **Plugin lifecycle (105):** curated-index discovery, metadata/provenance,
      install/enable/disable/remove/reload and persisted settings; exercise real
      sample plugins rather than a list or manifest parser alone.
- [ ] **Reporter/filter contracts (106):** attach/detach canvas drawing with
      document/view context; preview filters without mutation, commit once, cancel
      cleanly and reject stale previews; failed plugins must not corrupt history.

### Formats, export and hinting

- [ ] **Remaining codecs:** SVG fonts/glyphs, EPS background import, AFM/PFM,
      TTX subset, TTC/OTC writing, Type 42 and bare CFF; route supported inputs and
      outputs through user/CLI entry points, independently decode outputs and
      reject unsupported/malformed representations before replacing destinations.
- [ ] **Profiles/preflight (54, 58, 60, 63):** persist options, master/instance
      selection and profile matrix; share exact semantics with CLI; apply extrema,
      rounding, names, decomposition and subroutinization as supported; make
      preflight issues navigable and preserve source/destination on failure.
- [ ] **Hinting completion (61, 98 and P5 TT scope):** separately accept variable
      hint behavior, advanced Type 1 mechanisms and per-FD private data; extend
      authored TT source-resolution paths only with validated compiled bindings
      and independent interpreter tests. Preservation alone is not editable data.

### Inventory, layout, variables and bitmap parity

- [ ] **Inventory and search (10–19):** full badge/tag/list/search/filter
      semantics; shared text/visual predicate meaning; user glyph-data overlays and
      update flow; CJK fallback/generate-missing; batch rename updates references.
- [ ] **Metrics/layout (37–57, 99, 100):** full metrics grammar and all-master
      update; locked kerning/table maintenance; source inheritance; typed lookup
      editing, exact generator coverage, feature tokens/conditions and source maps.
- [ ] **Variable/CJK construction (55–57, 82, 97, 112):** main-canvas scrub,
      compatibility colors/connectors/drag repair, compound conditions, virtual
      masters, smart handles and radical/IDS/transform-grid workflows, with
      compatible export and explicit unsupported-topology errors.
- [ ] **Bitmap completion (65–71, 101, 103):** exact missing tool behaviors,
      overlay/HUD, integer-grid presets, configurable tracing, editable-pixel
      components, PNG sheets and pixel-outline detection; verify pixels/metrics
      across cross-strike copy/scale and native/interchange reopening.

### Shell, proofing and release

- [ ] **Shell/workspaces (1–9):** document-window and tear-off lifecycle,
      pinned/ordinal tab access, saved per-document layouts, preview divider,
      palette providers, scope-labelled history and the stated source layout.
- [ ] **Proofing (44, 73–76, 95, 109, 111):** stored grouped text, stepping,
      tri-state features/script/language/axes/clusters, instance/flip/blur and
      configurable text/glyph waterfall; export HTML/PDF and verify vertical
      origin/advance/layout in each applicable view/output.
- [ ] **Help/keymaps/localization (80, 107, 108):** contextual F1/hover help,
      imported named presets, conflict detection and actual command dispatch;
      independent Chinese translation/key/placeholder/accessibility verification.
- [ ] **Packaging/platform acceptance (9, 62 and P5 release):** build/install
      locally supported packages, inspect contents, smoke-test installed binaries,
      and collect actual macOS/Windows results through their platform runners.
      Unexecuted workflow definitions are recorded as pending execution. Native
      global-input automation remains prohibited.
- [ ] **Final integrated barrier:** current API generation, formatting, denied
      warning Clippy, relevant workspace/frontend/browser/independent checks;
      resource budget and owned-process cleanup; every audit row linked to its
      exact acceptance scope; user/developer docs and release limitations current.

## Supplemental accepted scopes and estimate consequence

The [MATH](p5-math-acceptance.md), [static SVG](p5-svg-acceptance.md),
[static COLRv1](p5-colrv1-acceptance.md) and [batch-style](p5-style-effects-acceptance.md)
records retain their accepted boundaries outside, or narrower than, the 112
rows. They must not be counted a second time merely because the checklist lacks
a dedicated row. Static SVG excludes richer embedded rendering features;
COLRv1 variable paints remain raw; batch styles are baked master-outline edits.

The effort-weighted central estimate remains **about 60%, with a 50–70% planning
range**, as explained in [the revised estimate](progress-estimate.md). Reusing
old evidence reduces ambiguity in the checklist, but those earlier working
paths were already credited by the original weighted estimate. New parallel
implementation changes the estimate only after integration and acceptance.

## Verification of this documentation task

No application builds/tests or native interaction were run for this review.
The documentation validation script passed with this output:

```text
PASS: all 112 IDs once; classifications {'P': 76, 'N': 20, 'R': 13, 'A': 3}; local links resolve
PASS: weights 100; historical weighted band 51.8–68.5; midpoint 60.15
```

Changed-file Prettier and `git diff --check` passed. The documentation resource
checkpoint ran `python3 scripts/check_artifacts.py`:

```text
build: 6.08 GiB, 13,158 files
temporary: 0.03 GiB, 2,697 files
workspace free: 669.24 GiB
PASS
```

The increase since task start belongs to concurrent shared implementation, not
this read-only review. No task scratch, server or browser process was created;
no other task's artifacts were removed. Only this report and
`progress-estimate.md` were edited by the reconciliation owner.
