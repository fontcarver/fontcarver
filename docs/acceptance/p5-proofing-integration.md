# P5 proofing integration acceptance

## Delivered scope

- `ProofLibraryPanel` persists named proof strings, groups, script/language and
  direction in project `FontInfo.user_data` through undoable `SetFontInfo`.
  Filters and previous/next buttons operate on the visible group; unmodified
  up/down in the text input steps saved strings through the editor transition
  barrier. A built-in kerning-pair sample replaces the old disabled placeholder.
- `TextShapingControls` exposes default/on/off feature states, custom tags,
  script/language overrides and source-cluster inspection. Script/language
  changes now hide geometry from the previous request during the debounce.
- `PreviewStrip` supports configurable text/single-glyph sizes, flip, inversion,
  blur, keyboard/pointer height control and shell detachment. `PreviewWindow`
  reconstructs document-backed shaping without installing another glyph editor.
- `ProofExportControls` exports complete shaped runs as standalone outline HTML
  or paginated vector PDF; the preview strip's bounded subset is never exported.
  Requests settle pending edits, reject document changes/missing glyphs, support
  cancellation and use a native save dialog or browser download.
- `VariableProofPanel` and `text.variable_preview` provide actual instantiated
  axis/instance proofing. Live shaper compilation now reuses a full baseline and
  incrementally replaces layout tables for eligible layout-only changes.
- `TextWorkspaceControls` integrates the optical-metrics owner's kerning mode,
  spacing side, group ghosts and kerning table controls. Color-rendering owner
  integration supplies preview and surrounding-glyph color paints.

## Focused evidence

Executed before root requested exclusive final integration:

- `pnpm exec vitest run src/views/text/ProofLibraryPanel.test.tsx src/views/text/prepareProofSheet.test.ts --maxWorkers=1 --testTimeout=30000`: 2 files, 5 tests passed.
- `pnpm exec vitest run src/views/text/useTextShape.test.tsx`: final combined run 3 tests passed (new script/language test first failed before fix).
- Library child: 14 tests passed; preview child: 10 tests passed; shaping-controls
  child: 6 tests passed; PDF/HTML child: 25 tests passed.
- Targeted ESLint for integration files passed; owned files Prettier formatted;
  `git diff --check` passed.
- PDF child rendered real quadratic outlines with Poppler and visually inspected
  both pages. See `docs/p5-proof-export-acceptance.md` for retained screenshots.

## Integration acceptance boundary

The user instructed finishing work already underway and then stopping. Root
subsequently reserved Cargo and final frontend checks because host load exceeded
130 and temporary-file count approached its budget. All queued subtree Cargo
commands were canceled and reported as interrupted, not passed. Root final tests
must cover `fc-doc --test proof_library`, `fc-api --test variable_proof`,
`fc-compile --test shaping_incremental`, layout-compile unit tests, and the full
frontend suite. Variable proof/component tests and incremental compiler tests
were authored but have not been executed by this owner. The final small kerning
sample integration and shared cross-owner TextView glue also require that run.

No new browser profile was started after root's resource hold. Integrated UI
browser acceptance is pending the fresh shared server: save/group/filter/step
proof strings; reopen `.fcf`; export HTML/PDF; toggle feature defaults and inspect
clusters; select single glyph/custom waterfall/flip/blur; detach/rejoin; exercise
mapped instance/axis previews and Off/On/Locked spacing guards.

HTML/PDF proof output uses outline paths. PDF visible non-ASCII labels use U+XXXX
fallback, while original text is retained in metadata/ActualText. Overwide PDF
runs are fitted to A4 with explicit requested-to-actual point-size annotation.
Native macOS/Windows execution and native save dialogs are not claimed from
Linux testing. No FontForge source was adapted, and no npm dependency was added.

## Root integration fixes

The root TypeScript check found two boundary errors: `prepareProofSheet` now
resolves a nullable master once through the document's first-master invariant,
and the proof-library test emits the typed `Document` event discriminator.

The root regression run exposed an active-glyph render loop. The newly wrapped
kerning keyboard callback changed identity on every render, causing `textRun`
to change and the glyph publisher to update its subscribed parent repeatedly.
The callback now retains its identity until its actual keyboard dependency or
selected gap changes. Its new regression failed before this fix. The shell
owner additionally narrowed GlyphView's publishing dependencies to the run
mode/presence it actually reads. Sibling proof panels now have distinct keys.

Executed `vitest run src/views/text/useTextKerningControls.test.tsx
src/views/text/TextView.master.test.tsx src/views/text/TextView.test.tsx
--maxWorkers=1 --testTimeout=30000`: **3 files, 11 tests passed**, 4.84 seconds.
