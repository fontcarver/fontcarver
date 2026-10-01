# p5f-h: Preview, proofing and proof export

Area id: `h-proofing`. Ports: API 8808, UI 1508. Protocol: `p5f-protocol.md`.

## Rows

| Row | Requirement                                                                                    | State at start                                                                                                       |
| --- | ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| 73  | Preview strip + detachable preview window; instance picker; invert; flip; blur                 | `PreviewStrip`, `PreviewWindow`; instance picker, flip and blur to verify/implement.                                 |
| 74  | Waterfall (text and single-glyph) with configurable steps                                      | Fixed waterfall exists; configurable steps and single-glyph mode to verify/implement.                                |
| 75  | Feature toggles (tri-state), script/language override, axis sliders, cluster inspector         | `TextShapingControls`; see `docs/acceptance/p5-proof-shaping-controls.md`; tri-state, overrides, clusters to verify. |
| 76  | Sample-string library in the project, grouped, script-aware                                    | `textSamples.ts`, `ProofLibraryPanel`, `proofLibrary.ts`; grouped project library to verify.                         |
| 95  | Pairs-and-phrases proof library with Up/Down stepping during spacing                           | Library exists; stepping during spacing to verify/implement.                                                         |
| 109 | Proof sheet export to PDF/HTML                                                                 | `proofExport.ts`, `ProofExportControls`; see `docs/p5-proof-export-acceptance.md`; verify PDF/HTML independently.    |
| 111 | [CJK] Vertical writing mode in all proofing surfaces (task l owns the Edit View and VORG part) | Vertical Edit View exists; preview strip, waterfall, proof export in vertical mode to verify/implement.              |

Existing records: `docs/acceptance/p5-proofing-integration.md`,
`docs/acceptance/p5-proof-shaping-controls.md`, `docs/p5-proof-export-acceptance.md`,
`docs/superpowers/plans/tasks/p5-proof-*.md`, `p5-proofing-integration.md`,
`docs/superpowers/reports/p5-proof-library-rust.md`, `docs/p5-cjk-vertical-acceptance.md`, `docs/p1-acceptance.md`.

## Scope notes

- Row 73: the strip and the detached window share state (text, instance,
  features); instance picker lists static instances and masters; invert, flip
  (horizontal/vertical) and blur (radius) render in Canvas2D; settings persist
  with the workspace (use the workspace store API owned by task a; do not edit it).
- Row 74: steps as a list or range with step size; text and single-glyph
  modes; per-step size label; export of the waterfall is part of proof export.
- Row 75: tri-state feature toggles (default/on/off) with the shaper honouring
  each; script and language override; axis sliders for variable fonts; a
  cluster inspector listing glyphs, clusters, advances and positions.
- Row 76/95: project-stored, grouped, script-aware library (Latin, Cyrillic,
  Greek, CJK, Arabic samples at minimum; pairs and phrases); Up/Down steps to
  the previous/next entry while spacing in the text view; import/export as text.
- Row 109: HTML export self-contained (embedded WOFF2 of the current export and
  CSS); PDF export with real embedded glyph outlines (not screenshots) via the
  existing `proofExport.ts`; verify the PDF with an independent parser (Python
  `pypdf` if installed, otherwise `qpdf --check`/`pdffonts` if available, or a
  minimal structural check) and the HTML by loading it in the headless browser.
- Row 111: vertical mode for the strip, waterfall, proof export (HTML and PDF)
  using the vertical metrics and `vert` features; evidence with a CJK fixture.

## Owned files

`app/src/views/text/**` except the kerning/spacing files owned by task d
(`kerningEdits.ts,KerningGhostControls*,kerningGhosts.ts,kerningImpact.ts,KerningModeControl.tsx,kerningModel.ts,kerningModes.ts,kerningQuery.ts,KerningRenameDialog.tsx,kerningTableData.ts,KerningTable*,SpacingControls*,TextGapPicker*,textGaps.ts,drawTextGaps.ts,textMetrics.ts,TextKerningPanel*,useTextKerning.ts,useTextMetrics.ts`)
and the shaper files owned by task e (`shapeCoordinator.ts,useTextShape.ts`);
`crates/fc-api/src/handlers/variable_proof.rs`, `crates/fc-api/src/types/variable_proof.rs`;
`crates/fc-doc/src/document/**` proof-library parts only (narrow); `app/e2e/text.spec.ts` (proofing cases);
`docs/p5f-h-proofing-acceptance.md`; user docs for proofing.

Shared files (narrow patches only): `app/src/views/text/{TextView.tsx,TextBar.tsx,textLayout.ts}`
(task d also patches these; keep hunks small), `crates/fc-doc/src/fcf/schema.md`,
`app/src/shell/{builtinCommands,shortcuts}.ts`, `app/src/stores/uiStore.ts`,
`app/src/i18n/en.json`, `crates/fc-api/src/handlers/mod.rs`, `crates/fc-api/src/types/mod.rs`.

## Gap analysis

Written before implementation, after reading the code of every file named above
and reproducing the open text-view regression in the headless browser. Each row
lists what already works, what is missing, what will be built, and what is
recorded as a limit.

### Row 73 — preview strip, detached window, instance picker, invert, flip, blur

Works: `PreviewStrip` renders positioned outlines at several pixel-per-em sizes
with a y-up Canvas2D transform, a per-size label, horizontal scrolling, a
pointer/keyboard height handle, collapse, a light/dark ink toggle (this is the
"invert": it flips preview ink and background independently of the application
theme), horizontal and vertical flip, a bounded 0–4 px Canvas2D blur, a
run/active-glyph switch, and a detach button that routes through
`detachPanel('preview')`. `PreviewWindow` rebuilds the shaping context from the
tab state without installing a second glyph editor.

Missing:

1. The strip and the detached window share only `previewTheme`,
   `previewCollapsed` and `previewHeight`. Flip, blur, the run/single switch and
   the size list are `useState` inside `PreviewStrip`, so the detached window
   does not follow the docked strip and nothing survives a workspace save.
2. There is no instance picker. Named instances are only reachable through
   `VariableProofPanel`, which renders a second strip and is hidden entirely for
   a font without axes.

Plan: move flip, blur, mode and the size list into `TextTabState`
(`previewFlipHorizontal`, `previewFlipVertical`, `previewBlur`, `previewMode`,
`previewSizes`, `previewInstance`), make `PreviewStrip` fully controlled, and
pass the same state from `TextView` and `PreviewWindow`. Add one instance picker
listing the design-space masters and the named instances: a master selection
re-shapes through the existing master path, an instance selection renders the
instantiated outlines from `text.variable_preview`. `TextTabState` is part of the
workspace snapshot, which `captureWorkspace` copies wholesale, so the only extra
change needed for persistence is additive validation in `readText`
(`app/src/stores/workspaceSnapshot.ts`).

Decision recorded: the task file says to use task a's workspace store API and not
to edit it. `readText` is the validator of `TextTabState`, a type this task is
allowed to extend in `uiStore.ts`; extending one without the other would store
settings that silently vanish on reopening. The patch is therefore additive —
new `...(condition ? { field } : {})` clauses in the existing style and nothing
else in that file — and is named in the final report.

### Row 74 — waterfall with configurable steps

Works: the comma-separated size list (1–8 unique sizes, 6–256 px) with
`aria-invalid`, an error message and rejection that keeps the last valid list;
per-step size labels; text and single-glyph modes.

Missing: the "range with step size" form the requirement names, and the waterfall
is not identifiable in the exported sheet because every row carries only the
sample name.

Plan: a new `waterfallSizes.ts` parser accepting both a list (`12, 24, 48`) and a
range with a step (`8-72/8`, and `8-72` at a default step), bounded and
deduplicated, with the parsed list shown back to the user; proof export labels
each row with its point size.

### Row 75 — tri-state features, script/language override, axis sliders, clusters

Works: `TextShapingControls` has a default/on/off selector per tag, custom tag
entry, script and language override fields with validation and Escape/Enter
semantics, and a glyph table. `VariableProofPanel` has real axis sliders and a
named-instance picker driving `text.variable_preview` against the kernel.

Missing: the cluster inspector lists one row per glyph with the source byte and
the two advances. It shows neither the cluster grouping (which glyphs belong to
one source cluster) nor the shaped x/y positions, both named by the requirement.
No kernel-level test asserts that "default" really leaves the shaper default
alone while "off" suppresses a feature that is on by default.

Plan: group the rows by source cluster, show the source text of each cluster, its
glyph list, and each glyph's position and advance; add an `fc-api` integration
test over `text.shape` that proves default/on/off behave differently on a feature
the font applies by default.

### Row 76 / 95 — grouped, script-aware project sample library with stepping

Works: `proofLibrary.ts` validates and stores named entries with group, script,
language and direction in `FontInfo.user_data` under one namespace, refuses
malformed or newer data without destroying it, filters by group/script/free
text, lists groups and steps with wrapping; `useProofLibrary` writes through
`withSettledDocument` and an undoable `SetFontInfo`, follows undo/redo, and
serialises concurrent writers; `ProofLibraryPanel` saves, updates, removes,
filters, steps with buttons and steps with bare ↑/↓ while the text field has
focus.

Missing:

1. The project library starts empty. There is no built-in grouped, script-aware
   sample set, so "Latin, Cyrillic, Greek, CJK, Arabic samples at minimum; pairs
   and phrases" is not satisfied by anything a user can reach. `textSamples.ts`
   only holds the all-encoded helper and `TextBar` has three hard-coded strings.
2. No import or export as text.
3. ↑/↓ stepping is bound to the text input only. While spacing, focus is on the
   canvas, so the row-95 gesture does not work where the row says it should.

Plan: a `proofSamples.ts` set of deterministic built-in entries (Latin pangram
and pairs and phrases, Cyrillic, Greek, CJK horizontal and vertical, Arabic),
inserted into the project library by one undoable action that skips ids already
present; TSV import/export through the shared artifact save path and a file
input, validated by the same `checkedEntry` bounds; and a capture-phase ↑/↓
handler on the text view that steps the visible list when the key did not come
from an editable field and the outline editor has no selection, so node nudging
is untouched.

### Row 109 — proof sheet export to PDF and HTML

Works: `prepareProofSheet` re-shapes every sample completely (never the strip's
bounded preview subset), waits out staleness, refuses compile failures, changed
documents and unresolved glyphs, fetches each outline once and multiplies samples
by sizes. `proofExport.ts` writes a standalone HTML file of inline SVG outlines
and a paginated A4 vector PDF with a real cross-reference table, quadratic to
cubic conversion, conservative bounds, `ActualText` metadata and an explicit
`[requested -> actual]` annotation when a run is fitted. `ProofExportControls`
settles the document, supports cancellation and saves through the native dialog.

Missing: the requirement asks for a self-contained HTML export with an embedded
WOFF2 of the current export and CSS. The current HTML is self-contained but is a
picture of the text, not text: it cannot be selected, searched or re-flowed, and
it does not prove the exported font works in a browser. No API returns compiled
font bytes to the frontend (`session.export` writes to a path), and the exported
rows do not name their point size.

Plan: add `text.proof_font` to the owned `variable_proof` handler and types,
returning base64 WOFF2 compiled from the live document at the selected master or
named instance, together with the family name and units per em; add a rendering
choice to the export controls — "Embedded font" (an `@font-face` with the WOFF2
as a data URI, CSS, real text, `lang`, `dir` and `font-feature-settings` from the
proof's own shaping context) and "Outlines" (today's exact shaped snapshot) —
defaulting HTML to the embedded font and keeping PDF on outlines, which is what
"real embedded glyph outlines, not screenshots" asks for. Verify the PDF with
Poppler (`pypdf` and `qpdf` are not installed here; `pdfinfo`, `pdffonts`,
`pdftotext` and `pdftoppm` are) and the HTML by loading it in headless Chrome.

### Row 111 — vertical writing mode in the proofing surfaces

Works: `previewGeometry` detects a vertical run from a non-zero `yAdvance`,
grows the canvas, and `paintPreview` places the baseline from the run's top;
`buildTextLayout` forces `vert` and `vkrn` on for a `ttb` request and strips them
otherwise; `proofExport.prepare` includes vertical advances in its bounds.

Missing: evidence. No test or browser run shows a vertical strip, a vertical
waterfall or a vertical proof sheet, and the exported HTML writes no
`writing-mode`, so the new embedded-font rendering would set vertical CJK text
horizontally.

Plan: vertical-run unit tests for the strip geometry and both exporters,
`writing-mode: vertical-rl` plus `text-orientation` in the embedded-font HTML for
a `ttb` entry, and browser evidence on a CJK fixture. The large external Source
Han Sans corpus needs a release kernel, so the browser evidence uses a small
vertical CJK subset built into the task scratch directory with `pyftsubset` and
is documented as such; nothing is committed.

### Open regression `app/e2e/text.spec.ts:122` — root cause

Reproduced on ports 8808/1508: the case failed 2 of 6 runs, always on
`expect(view).toHaveAttribute('data-run-count', '15')` with the received value
`0`.

`docs/TODO.md` attributes it to `TextView.transition()` dropping its `patch()`
when the editor instance changes between its awaits. That is **not** the cause.
Instrumenting `TextView`, `useTextShape` and `TextShapeCoordinator` (temporary
dev-only logs, since reverted byte-for-byte) shows, in a failing run:

- `state.text` is exactly the typed string, so the `patch()` was applied and no
  typed text was lost;
- `editing` is false, the coordinator is alive and undisposed, and
  `coordinator.update()` is called with the typed text;
- `execute()` issues the `POST /api/text.shape` request;
- the response arrives **3.9–4.3 s** later, and `useTextShape` returns `null`
  until it does, which renders `data-run-count="0"`.

Measured: a cold kernel answers the first `text.shape` on the DejaVu fixture in
about 4 s, because that request compiles the whole font for the live shaper; a
warm kernel answers in 79 ms. `playwright.config.ts` has
`reuseExistingServer: !process.env.CI`, so a run that reuses a server left over
from a previous run gets the warm path and passes, and a run that starts its own
kernel gets the cold path and races Playwright's 5 s `expect` timeout. The second
reported symptom — the click that should enter local edit mode doing nothing —
follows from the same cause: `activate()` refuses an index that the unshaped run
does not have yet.

So the defect is a missing distinction, not lost data: the view publishes
`data-run-count="0"` both while it is waiting for the shaper and when the text
really produced no glyphs, and the spec asserts the ambiguous value with the
default timeout. Fix: publish `data-text-shaping` on the text view root beside
the run count (the shell already shows this state in `ProgressLine`, so no new
behaviour is invented), and make the spec wait for the first compile explicitly,
with a comment naming the measured cost. The kernel holding the document while
it compiles is real but belongs to the shaping owner; it is reported, not
changed here.
