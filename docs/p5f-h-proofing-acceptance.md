# p5f-h acceptance: preview, proofing and proof export

Task: `superpowers/plans/tasks/p5f-h-proofing.md`. Audit rows **73, 74, 75, 76,
95, 109, 111** (the proofing half of 111; the Edit View and `VORG` half is task
`l`). Ports API 8808 / UI 1508. Date 2026-10-01. Protocol: `p5f-protocol.md`.

The gap analysis behind this report — what already worked, what was missing and
why each decision was taken — is appended to the task file. User documentation is
[Previewing and proofing text](user/proofing.md).

## What was built

**Row 73 — preview strip, detached window, instance picker, invert, flip, blur.**
The strip already had inversion (its own light/dark ink, independent of the
application theme), horizontal and vertical flip, a bounded 0–4 px Canvas2D blur,
a run/active-glyph switch, a resizable height and shell detachment — but flip,
blur, the content switch and the size list were component state, so the detached
window did not follow the docked strip and nothing survived a workspace save.
`PreviewStrip` is now fully controlled: `previewSettings.ts` maps those settings
to and from `TextTabState`, `TextView` and `PreviewWindow` pass the same values,
and `workspaceSnapshot.readText` validates them so a saved workspace reopens with
the same waterfall. The strip also gained the missing **instance picker**:
`usePreviewInstance` lists every design-space master and every named instance:
choosing a master moves the whole run (the editor, metrics and kerning follow),
and choosing a named instance renders the interpolated outlines
`text.variable_preview` returns for it without touching the edited master.

**Row 74 — waterfall with configurable steps.** `waterfallSizes.ts` accepts both
forms the requirement names: a list (`12, 24, 48`) and a range with a step
(`8-32/8`, `8..32`, `8 to 32`, and a bare range at the smallest whole step that
fits eight rows). Steps are bounded to 6–256, deduplicated and sorted, the field
is replaced with the list actually drawn, and a rejected draft keeps the drawn
waterfall and says what is wrong. Each step keeps its size label, text and
single-glyph modes both work, and the proof sheet now names each row's point
size, so an exported waterfall reads as one.

**Row 75 — tri-state features, overrides, axis sliders, cluster inspector.** The
tri-state selectors, the script and language overrides and the axis sliders were
present; the inspector was a flat per-glyph list. `textClusters.ts` groups the
run by source cluster and the panel now shows, per glyph, the cluster ordinal in
visual order, the UTF-8 source offset, the source characters, the glyph name and
both the position and the advance. A base and its marks share one cluster, and a
right-to-left run reads in drawing order while its offsets still name the source.
A new kernel test proves the three feature states actually differ.

**Rows 76 and 95 — grouped, script-aware project library with stepping.** The
project library existed but started empty, could not be exchanged as text, and
its ↑/↓ stepping only worked in the text field. `proofSamples.ts` adds 21
built-in samples in six groups across five scripts — Latin, Cyrillic and Greek
pangrams, uppercase, lowercase, figures and diacritics; capital, lowercase and
punctuation kerning pairs and spacing control words; Han phrases including a
vertical one; Arabic phrases and joining forms — adopted into the project by one
undoable command with deterministic `fc.sample.*` identifiers, so adopting twice
adds nothing and an edited copy is never replaced. `proofInterchange.ts` exports
and imports the library as one tab-separated line per entry; an import matches by
group and name so a corrected file updates rather than duplicates, and a document
with one malformed line imports nothing. `proofStepping.ts` and a capture-phase
handler on the text view make bare ↑/↓ step the visible list while the canvas has
focus — which is where spacing happens — without taking the arrows away from a
selected node or from any modified chord.

**Row 109 — proof sheet export.** The sheet already re-shaped every string
completely and wrote vector HTML and PDF. The requirement's self-contained HTML
"with the embedded WOFF2 of the current export and CSS" was missing, because no
API returned compiled font bytes to the browser. `text.proof_font` (new, in the
owned `variable_proof` handler) compiles the selected master or named instance
and returns the WOFF 2 base64-encoded with its family and style name and units
per em; `exportProofHtml` can now write an `@font-face` page that sets the real
text in that face, with the proof's own `dir`, `lang` and
`font-feature-settings`. The outline rendering remains available and unchanged,
and is what PDF always uses.

**Row 111 — vertical mode in the proofing surfaces.** Vertical runs already
reached the strip's geometry and the exporter's bounds; what was missing was the
embedded-font sheet (now `writing-mode: vertical-rl` with upright orientation for
a `ttb` entry) and any evidence. Both are now covered by unit tests and by a
browser run on a vertical CJK subset.

**The open regression `app/e2e/text.spec.ts:122`.** Diagnosed, fixed and
documented below.

## The text-view regression: measured cause

`docs/TODO.md` records this as "typed text is sometimes dropped before it is
shaped", attributed to `TextView.transition()` dropping its `patch()` when the
editor instance changes between its awaits. Reproduced here on ports 8808/1508:
the case failed **2 of 6** isolated runs, always on
`expect(view).toHaveAttribute('data-run-count', …)` receiving `0`.

Temporary dev-only instrumentation of `TextView`, `useTextShape` and
`TextShapeCoordinator` (since reverted byte-for-byte; `git status` shows no
change to the two files owned by task `e`) recorded, in a failing run:

- `state.text` was exactly the typed string, so the `patch()` had been applied
  and no typed text was lost;
- `editing` was false, the coordinator was alive and undisposed, and
  `coordinator.update()` was called with the typed text;
- `execute()` issued `POST /api/text.shape`;
- the response arrived **3.9–4.3 s later**, and `useTextShape` returns `null`
  until it does, which renders `data-run-count="0"`.

Measured directly: the first `text.shape` of a session compiles the whole fixture
for the live shaper and takes about 4 s on a debug kernel here; once warm the same
request takes 79 ms. `playwright.config.ts` sets
`reuseExistingServer: !process.env.CI`, so a run that inherits a warm kernel
passes and a run that starts its own races Playwright's 5 s `expect` timeout. The
second reported symptom — the click into local edit mode doing nothing — follows
from the same cause: `activate()` refuses an index the unshaped run does not have.

So nothing was dropped; the defect was a missing distinction. The text view now
publishes `data-text-shaping` beside `data-run-count`, so "waiting for the
shaper" and "this text produced no glyphs" are different states to a reader, a
test and the status region (the shell already showed this state in
`ProgressLine`), and the spec waits for the first compile explicitly with a
comment naming the measured cost. `TextView.transition()` was **not** changed:
the evidence does not support the hypothesis recorded in TODO, and changing a
barrier that four other hooks depend on without a defect to point at would be a
guess.

## Commands and results

Rust, under the shared Cargo lock with an absolute task `TMPDIR`:

```text
cargo test -p fc-api --test proof_shaping
  running 4 tests
  test proof_font_rejects_an_unknown_master_or_instance_without_touching_the_document ... ok
  test feature_override_states_are_default_on_and_off ... ok
  test proof_font_compiles_the_default_master_as_woff2 ... ok
  test proof_font_compiles_a_named_instance_and_each_master ... ok
  test result: ok. 4 passed; 0 failed; 0 ignored

cargo clippy -p fc-api --no-deps --lib --bins --tests -- -D warnings   → clean
rustfmt --edition 2021 <3 owned files>                                 → clean
flock .scratch/p5f/cargo.lock pnpm --dir app gen:api                   → ProofFont, ProofFontParams
```

`cargo clippy -p fc-api` **with** dependencies fails inside `fc-sfnt`
(`manual_is_multiple_of`, two sites) and `fc-formats`
(`type_complexity` in `type1/hints.rs:617`). Neither crate is touched by this
task; `--no-deps` is the run that lints this task's own code.

Frontend, from `app`:

```text
pnpm exec vitest run --maxWorkers=1 --minWorkers=1 src/views/text/ --testTimeout=30000
  Test Files  1 failed | 52 passed (53)
       Tests  1 failed | 333 passed (334)
```

The single failure is `KerningTable.test.tsx` (task `d`'s file), which asserts
`getByText('Class')` while another agent has changed `kerningTable.classRule`
from "Class" to "Group" in `en.json`. It is unrelated to this task and is
reported, not changed. Every file this task owns or added passes; the new and
rewritten ones are:

```text
pnpm exec vitest run --maxWorkers=1 --minWorkers=1 <the 14 files of this task> \
  --testTimeout=30000
  Test Files  14 passed (14)
       Tests  119 passed (119)

waterfallSizes 8 · proofInterchange 10 · previewSettings 9 · proofStepping 4 ·
textClusters 7 · usePreviewInstance 5 · PreviewWindow 2 · PreviewStrip 12 ·
proofExport 31 · prepareProofSheet 4 · ProofLibraryPanel 6 · useProofLibrary 8 ·
TextShapingControls 6 · TextView 7
```

```text
pnpm check (tsc -b)                        → no error in any file of this task
pnpm exec eslint src/views/text/ e2e/text.spec.ts src/stores/uiStore.ts \
  src/stores/workspaceSnapshot.ts          → clean
pnpm exec prettier --check <owned files>   → all matched files use Prettier style
```

`pnpm check` over the whole app still reports errors in `src/dialogs/*`,
`src/stores/exportProfileStore.test.ts`, `src/transport/HttpTransport.test.ts`,
`src/test/fontFixtures.ts` and the hint editors: the regenerated `ExportOptions`
now carries `tt_autohint` and `PsHints` carries `flex_hints`/`dot_sections`,
which task `k`'s callers have not caught up with. Running `gen:api` is this
task's doing (the protocol requires it after an `fc-api` type change) but the
unfinished callers are not.

End-to-end, against the task's own kernel and Vite on 8808/1508:

```text
FC_API_PORT=8808 FC_UI_PORT=1508 pnpm exec playwright test e2e/text.spec.ts
  ✓ shapes text and edits the third glyph in local coordinates (12.8s)
  ✓ configures the waterfall as a list and as a range with a step (12.2s)
  ✓ adopts the built-in samples and steps them from the field and the canvas (4.1s)
  ✓ exports a self-contained HTML proof with the compiled font embedded (26.4s)
  ✓ exports a vector PDF proof and a vertical outline sheet (3.8s)
  ✓ previews a named instance and a second master from the strip (5.3s)
  6 passed (1.1m)
```

The first case is the previously intermittent one; it passed every run after the
explicit shaping wait, including six consecutive isolated runs during diagnosis.

## Browser and independent output evidence

Screenshots in `screenshots/p5f-h-proofing/`, all captured from the running
application on 8808/1508:

- `preview-waterfall.png` — the strip after `10-72/16` was entered as a range:
  four labelled steps (10, 26, 42, 58 px), 0.5 px blur and horizontal flip, with
  the field replaced by the resolved list.
- `shaping-clusters.png` — tri-state selectors (`kern` On, `liga` Off, `calt` On,
  the rest Default), both override fields, and the cluster inspector showing
  cluster, source byte, source character, glyph, X/Y position and both advances.
- `proof-library.png` — the library after **Add built-in samples**: 21 saved
  strings, group and script filters, the stepping buttons, the interchange
  buttons and the proof-sheet controls including **HTML rendering**.
- `embedded-html.png` — the exported HTML opened as a local file, its two rows
  set at 18 pt and 48 pt in the embedded WOFF 2.
- `vertical-cjk-strip.png` — the strip on a vertical CJK run (永東國書道) at
  16/32/64 px, glyphs stacked downwards.

Independent checks of the emitted files, all produced by the application's own
export buttons through a Playwright download:

```text
pdfinfo proof.pdf
  Title: DejaVu Sans Book · Pages: 1 · Page size: 595.276 x 841.89 pts (A4)
  PDF version: 1.7 · JavaScript: no · Encrypted: no
pdffonts proof.pdf
  Helvetica  Type 1  Standard  emb:no   (the labels; the specimens are outlines)
pdftotext proof.pdf -
  DejaVu Sans Book / Hamburgefonstiv / 24 pt / Hamburgefonstiv / 48 pt / 1
pdftoppm -r 110 -png proof.pdf
  [both waterfall rows inspected: correct counters, curves and baselines]
```

The embedded-font HTML was loaded in headless Chrome by the spec itself: after
`document.fonts.ready`, `document.fonts.check('48pt FCProofFont')` is true, the
computed family is the embedded one, the computed sizes are `24px` and `64px`
(18 pt and 48 pt), the text is the proofed string, and the page contains no
`<svg>`, `<script>`, `<img>` or `<link>` — it is self-contained.

The compiled face itself was decoded independently with fontTools:

```text
magic b'wOF2'  bytes 280972  flavor woff2
numGlyphs 6253  unitsPerEm 2048  name1 'DejaVu Sans'  name2 'Book'
cmap 5918 entries, 'A' -> 'A'
```

Vertical CJK, on a scratch subset of the external Source Han Sans SC (23 glyphs,
`vmtx`/`VORG`/`vert` retained, built with `pyftsubset`, **not committed**):

```text
text.shape ttb  → names uni6C38 uni6771 uni570B uni66F8 uni9053
                  advances (0, -1000) ×5
cjk-vertical.html → @font-face present, writing-mode:vertical-rl present,
                    specimen class "specimen vertical", text 永東國書道
pdfinfo/pdftotext cjk-vertical.pdf → Source Han Sans SC Regular / 永東國書道 / 36 pt
pdftoppm                            → the five Han glyphs stacked vertically
```

`pypdf` and `qpdf` are not installed on this machine; Poppler's `pdfinfo`,
`pdffonts`, `pdftotext` and `pdftoppm` are, and are what the scope note names as
the alternative.

## Kernel-level checks of invalid input and locks

- `text.proof_font` refuses an out-of-range named instance, `u32::MAX`, and an
  unknown master, and the document is byte-identical afterwards
  (`proof_font_rejects_an_unknown_master_or_instance_without_touching_the_document`).
- `feature_override_states_are_default_on_and_off` shapes with the tag absent,
  forced on and forced off over **two** features: `liga`, which the font applies
  by default (Default = On, Off differs), and `dlig`, which it does not
  (Default = Off, On differs). Neither feature alone separates three states; the
  two together do, which is the case a Boolean toggle cannot make.
- The proof library keeps its existing kernel-side guarantees and gains two more
  entry points through the same barrier: `adopt` and `importText` both run inside
  `withSettledDocument`, read the font info fresh, merge, and write one undoable
  `SetFontInfo`; a malformed import throws before any write, so the stored
  library is unchanged and the specific reason is shown
  (`useProofLibrary` tests, `proofInterchange` tests).
- `prepareProofSheet` still refuses a changed document, a compile failure, a
  stale layout and unresolved glyphs before producing anything, and the new
  embedded-font path compiles the face **inside the same barrier** as the shaping
  and re-checks the generation, so the font and the text it sets come from one
  revision.
- The embedded-font HTML is an injection surface, so it is treated as one: the
  base64 is validated, the CSS family name is a fixed internal token rather than
  the font's own name, feature tags that could close the CSS string are dropped,
  the language tag is validated, and every value is HTML-escaped. One of these
  (a tag containing a quote closing the `style` attribute) was a real defect
  found by its own test before it shipped.

## Remaining limits

1. **Two proofing surfaces for the design space.** The strip's picker covers
   masters and named instances; arbitrary axis locations stay in the separate
   **Variable font proof** panel, which keeps its own waterfall settings so
   changing them does not move the docked strip. Merging the two into one panel
   is a layout decision, not a missing capability.
2. **PDF labels are not font-embedded.** They use the built-in Helvetica, so a
   non-ASCII label character is shown as `U+XXXX` (now separated by spaces so a
   CJK label is readable) with the original text kept in `ActualText` and the
   title metadata. The specimens themselves are outlines and cover every script.
3. **The embedded-font HTML is shaped by the viewer's browser**, not by our
   shaper; it carries the same features, language and direction, which is what
   makes it a useful check of the exported font, but the outline rendering
   remains the exact record of what the editor drew. Both are offered and
   documented.
4. **The strip's canvases keep a 160 px minimum width**, so a vertical waterfall
   step below about 32 px is still wider than it is tall. This is a legibility
   floor, not a vertical-layout defect; the larger steps show the real aspect.
5. **No committed variable or CJK fixture.** The instance-picker browser case
   authors a two-master design space with one named instance on
   `fixtures/ufo/Minimal.ufo` through the same `variation.apply` the design-space
   editor uses, and closes only the document it opened. The vertical CJK evidence
   uses a 23-glyph scratch subset of the external Source Han Sans SC; it is not
   committed and the acceptance above records exactly what it showed.
6. **The selected proof string is session state.** `proofId` lives on the tab so
   stepping continues where it left off, but it is deliberately not restored from
   a saved workspace: the entry it names may have been renamed or removed.
7. **Colour proofing is unchanged.** The strip renders the static palette-zero
   paints the colour owner supplies; axis-dependent paints and colour proof sheets
   remain out of scope, as `TODO.md` records.
8. **Native save dialogs are not exercised.** Every export here went through the
   browser download path; the Tauri branch of `saveArtifact` is unchanged and
   unverified from Linux, as earlier records state.

## Shared files and interfaces

Narrow additive patches only, each re-read immediately before editing:
`crates/fc-api/src/api.rs` (one method row, one dispatch arm),
`crates/fc-api/src/bin/gen_ts.rs` (two export calls),
`crates/fc-api/Cargo.toml` (the `base64` workspace edge),
`crates/fc-doc/src/fcf/schema.md` (the proof-library namespace, previously
undocumented), `app/src/stores/uiStore.ts` (six optional `TextTabState` fields),
`app/src/stores/workspaceSnapshot.ts` (validation for those fields in `readText`;
see the decision recorded in the task file's gap analysis),
`app/src/i18n/en.json`, `THIRD_PARTY.md`, `docs/user/spacing-and-kerning.md`
(one cross-reference).

New English key namespace: `proof.samples` (`groups.*`, `names.*`). Additions
inside existing namespaces: `proof.library.{undoAdopt,undoImport,adoptSamples,
exportText,importText,interchangeFilter,importFailed}`,
`proof.export.{rendering,renderingFont,renderingOutlines,renderingHint,
errors.font}`, `proofPreview.{instance,masters,instances}` and a reworded
`proofPreview.sizesHint`, `proofShaping.{cluster,source,glyphs,x,y}`.

New API: `text.proof_font` (`ProofFontParams` → `ProofFont`). New dependency
edge: `fc-api` → the already locked workspace `base64` 0.22 (`MIT OR
Apache-2.0`), recorded in `THIRD_PARTY.md` §3.1. No npm dependency was added and
no FontForge source was read or adapted.

## Per-row recommended status

| Row | Recommendation                   | Basis                                                                                                                                                                                                         |
| --- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 73  | **Complete**                     | Shared, persisted strip and detached window; instance picker over masters and named instances; invert, flip and blur; browser and component evidence.                                                         |
| 74  | **Complete**                     | List and range-with-step forms, bounded and round-tripped; text and single-glyph modes; per-step labels; sizes named in the exported sheet.                                                                   |
| 75  | **Complete**                     | All three feature states separated by the kernel test, over one default-on and one default-off feature; script and language overrides; axis sliders; cluster inspector with clusters, positions and advances. |
| 76  | **Complete**                     | 21 grouped, script-aware built-in samples adopted into the project as one undo entry; filters; text import and export; `.fcf` persistence documented.                                                         |
| 95  | **Complete**                     | Pairs, spacing words and phrases in the library; ↑/↓ stepping in the field and on the canvas while spacing, without taking the arrows from a node.                                                            |
| 109 | **Complete**                     | Self-contained HTML with the embedded WOFF 2 and CSS, or exact outlines; vector PDF; verified with Poppler, fontTools and a headless browser load.                                                            |
| 111 | **Complete** (proofing surfaces) | Vertical strip, waterfall and both sheet renderings on a CJK fixture. The Edit View and `VORG` half of the row is task `l`'s.                                                                                 |

## Review round

Five defects came back as ACCEPT WITH FIXES. Each is fixed with a test that fails
without the fix.

**1 (medium, row 74) — the field rejected its own example.** `8-72/8` expands to
nine steps and `checked()` refused anything over eight, so the hint and
`docs/user/proofing.md` advertised an entry the field would not take.
`parseWaterfallSizes` now **cuts a range** to the first `MAX_WATERFALL_STEPS` and
reports `truncated`, while a **list** of more than eight is still refused —
a range states an intention that survives not fitting, an explicit list does not.
The strip shows the note beside the field and keeps the entry valid.
Tests: `waterfallSizes.test.ts` "cuts a long range to the steps a waterfall shows
and says it did" (`8-72/8` → eight steps, `truncated` true; `6-256/1` likewise)
and "refuses an over-long list rather than shortening an explicit choice";
`PreviewStrip.test.tsx` "draws the first eight steps of a long range and says
that it cut it" asserts the note, `aria-invalid="false"` and eight rendered rows.

**2 (medium, row 76) — the interchange lost entries and trailing spaces.** An
exported line whose group began with `#` was read back as a comment and skipped,
so three entries exported and two imported with no error, contradicting this
report's own "imports nothing rather than half"; and `raw.trimEnd()` stripped
trailing spaces from the text, so the spacing string `HH HOH ` did not round-trip
and a text of only spaces aborted the whole import. `escapeField` now escapes a
leading `#` (`\#`, accepted on the way back by a new `UNESCAPES` entry) and
nothing else, so `C# F#` stays readable; the importer strips only a stray `\r`
and skips a line as blank only when it has no tab at all.
Tests: `proofInterchange.test.ts` "round-trips an entry whose group or name
starts with a hash" (all three come back, `\#Draft` is escaped, `C# F#` is not),
"round-trips a text made only of spaces", and the renamed "ignores comments and
blank lines but keeps a text's trailing spaces".

**3 (low) — one message for every rejection.** `parseWaterfallSizes` now returns
a `WaterfallReason` (`empty`, `token`, `range`, `bounds`, `duplicate`, `count`)
and the strip shows the keyed message for it instead of restating the grammar.
Tests: `waterfallSizes.test.ts` "names why an entry was refused" over twelve
entries; `PreviewStrip.test.tsx` "says which rule an entry broke instead of
restating the grammar" walks six entries through the field and then clears it.

**4 (low, row 75) — the kernel test proved two states, not three.** `liga` is on
by default, so `on == default` and only `off` differed. The fixture gained a
`dlig` feature (and an `A_V` ligature) that the shaper leaves **off** by default,
so Default = Off and On differs — the mirror image of `liga`. The test now
asserts both pairs and that `liga` Default ≠ `dlig` On, which no single Boolean
can reproduce. The row-75 sentence in the per-row table and in the invalid-input
section is corrected to say exactly this.

**5 (low) — the stepping hint named only the text field.**
`proof.library.stepping` now says "in the text field, or on the canvas while
spacing", matching what row 95 actually ships.

### Keys changed in `app/src/i18n/en.json`

For the translation pass: **changed** `proofPreview.sizesHint` (rewritten, since
the step limit now applies to lists rather than to every entry) and
`proof.library.stepping` (mentions the canvas). **Added**
`proofPreview.sizesTruncated` (uses `{{count}}`) and
`proofPreview.sizesError.{empty,token,range,bounds,duplicate,count}`. No key was
removed or renamed.

### Verification of the review round

```text
cargo test -p fc-api --test proof_shaping                            → 4 passed, 0 failed
cargo clippy -p fc-api --no-deps --lib --bins --tests -- -D warnings → clean
rustfmt --edition 2021 crates/fc-api/tests/proof_shaping.rs          → clean

pnpm exec vitest run --maxWorkers=1 --minWorkers=1 <the 14 files of this task>
  Test Files  14 passed (14)
       Tests  124 passed (124)      (119 before the round; +5 new cases)

pnpm exec playwright test e2e/text.spec.ts                           → 6 passed (1.3m)
pnpm check                          → no error in any file of this task
pnpm exec eslint <owned dirs>       → clean
pnpm exec prettier --check <owned>  → all matched files use Prettier style
```

## Resource and process state

`python3 scripts/check_artifacts.py` at task start: build 6.16 GiB / 15,007
files, temporary 0.00 GiB, free 627.12 GiB, **PASS**. After the review round:
build 12.73 GiB / 19,147 files, temporary 0.05 GiB / 641 files, free 619.08 GiB,
**PASS with the 10 GiB build-cache warning**. The shared cache is six agents
compiling at once — it fell to 6.91 GiB between the two rounds when another agent
cleaned it — and no other task's cache or scratch was removed by this one.

This task's kernel (port 8808) and Vite (port 1508) were stopped, its scratch
directory `.scratch/p5f-h-proofing/` — including the generated CJK subset, the
exported sheets and their rendered pages — was deleted, and `app/test-results/`
was removed. The only retained artefacts are this report and the five screenshots
it cites. The documents the browser cases opened (`fixtures/ufo/Minimal.ufo` and
the CJK subset) are closed by the cases themselves, and the proof library the
sample case adopts is restored to the metadata it found.
