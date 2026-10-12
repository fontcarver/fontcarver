# p5f-c acceptance: outline editor tools, interaction and glyph panels

Date: 2026-09-30. Task: `superpowers/plans/tasks/p5f-c-editor.md` (audit rows 20–36,
81, 89–91). Ports 8803 (API) / 1503 (UI). Protocol: `p5f-protocol.md`.

This report records what this task verified and what it implemented. It does not
speak for any other `p5f-*` area, and it claims nothing that is not backed by a
command whose output is quoted below.

## Scope

The 2026-09-30 batch had already implemented most of these rows and left four
kinds of hole: Rust targets that were written but never executed; rows whose
requirement text named something that genuinely did not exist; a row whose flag
was stored and documented but never acted on; and rows with unit tests but no
evidence from a running application. The per-row gap analysis, including the
three assessments this task had to correct while implementing, is in the task
file.

## What was implemented

### Row 20 — the Pixel tool and the command palette

`Pixel` (`X`) now exists in the tool table, drawn with its own icon, labelled
and hinted. Choosing it — from the strip, by its key, or by holding its key —
persists the open editor, opens the bitmap workspace for the same document and
restores the previous tool, so the Edit tab is never left holding a tool with no
behaviour. Pixel drawing itself belongs to the strike editor (task `g`).

Every tool is also a command now (`tool.<id>`), so the command palette reaches
all sixteen entries of the strip. The commands deliberately carry no shortcut of
their own: the tap/hold grammar needs the key down *and* the key up, which
one-shot command dispatch cannot express, so the key stays with
`useToolShortcuts` and the palette shows the live binding and the gesture hint
as the command's detail. A palette-only command had no way to show a detail
before, so `usePaletteEntries` now falls back to `command.detail()` when a
command names no menu; a command that does name one is unchanged.

The registry's doc comment claimed the Text tool was inert. It is not: in an
Edit tab, choosing Text switches the tab into its shaped-text mode
(`TextView`). The comment was corrected rather than the behaviour.

### Row 27 — Open Corner and Reconnect Nodes

Both were recorded as unimplemented by `p5-path-operations-acceptance.md`. They
now exist at all three levels:

- `fc_geom::open_corner(contour, node, depth)` replaces one corner node with two
  nodes placed along the two adjacent tangents *past* the corner, joined by a
  straight mouth, so the contour crosses itself in a triangle. The moved node
  keeps its stored identity and its incoming segment's kind; the second node is
  fresh. Both adjacent handles are untouched, so neither tangent changes.
- `fc_geom::reconnect_nodes(contour, a, b)` is the exact inverse: the two ends of
  one segment merge at the intersection of the incoming tangent at `a` and the
  outgoing tangent at `b`, keeping `a`'s identity, and the mouth's controls go
  away. A mouth that spans a closed contour's stored start is handled without
  losing the start point.
- `Command::OpenCorner` and `Command::ReconnectNodes` route through the existing
  outline handler, so each is one undo unit, each is rejected atomically, and
  each survives `.fcf` write and read.
- The Path menu gains **Open Corner** (`Mod+Alt+O`) and **Reconnect Nodes**
  (`Mod+Alt+J`). The setback is computed from the glyph: the smaller of one
  fiftieth of the em and a quarter of the shorter adjacent chord. Reconnect
  Nodes also covers the other reading of the name — two open contour ends are
  joined through the existing `JoinContours`.

Rejected input: an off-curve or out-of-range node, an endpoint of an open
contour, coincident neighbours, a non-positive or non-finite setback, a setback
that is not shorter than both adjacent chords, and a node that still carries a
live smart corner. Reconnect additionally rejects two nodes that are not one
segment apart, are not in one contour, have parallel tangents, meet on the wrong
side, or would collapse onto a neighbour.

**Documented limit.** Only a concave (reflex) corner can cross its own contour,
so removing the overlap of an opened *convex* corner leaves the triangle as a
separate ear rather than restoring the corner. The construction is still exactly
reversible, which is what Reconnect Nodes is for, and both behaviours are
asserted by name in `crates/fc-geom/tests/corner.rs`.

### Row 30 — LSB/RSB guide anchoring

`Guide.anchor` (`Origin` | `Lsb` | `Rsb`) is new in `fc-model`, documented in
`fcf/schema.md`, and defaulted so every existing document and every imported
guide reads back as `Origin`. `Origin` keeps `x` absolute; `Lsb` and `Rsb` make
it a signed offset from the left or right edge of the layer's ink, so the guide
follows the drawing. The scene resolves the anchor into a position once, which
is why drawing, hit testing, snapping and measurement needed no changes at all.
The Info box and the Guides panel both offer the selector and both rebase the
stored offset when it changes, so the line never jumps. A blank layer has no ink
edge, so an anchored guide there behaves as absolute.

### Row 53 (task `f`) — the font's own grid and increments

Grid spacing, its subdivision and the arrow-key increments are stored per font
and resolved by `fontinfo.editor_settings_get`. The editor now reads that view
once per document (and again on each font generation) and hands it to
`snapSettingsOf`/`nudgeStepsOf`, which apply the order in one place: an override
that is present wins, anything absent leaves the preference in charge, and zero
still means "no grid" whichever side set it. Grid drawing and snapping both read
the resolved step, because both already read `SnapSettings.gridSize`. A read
that fails leaves the preference in charge rather than stopping the edit.

### Row 31 — one distance per snap target

`SnapPrefs` gains a `distances` record with one value per target. Zero means
"use the shared snap distance", so a preference file that has never been edited
behaves exactly as one radius did; the kernel validates each value as finite and
within 0–64 CSS pixels. The snapping engine now resolves a reach per family,
including the alignment lines derived from nodes and handles, and Preferences →
Snapping puts a distance field beside each of the nine switches.

### Row 32 — Clear Background

`layer.clearBackground` empties the current master's background layer in one
batch (its contours and its components) and keeps the layer, so the next copy
starts from a clean sheet. It is in the Layer menu and in the Layers panel, and
it disables itself with a reason when there is no background ink.

### Row 35 — a live audit with more checks and more fixes

- The audit now follows the geometry the canvas is drawing. During a gesture it
  reads the optimistic preview instead of the deliberately stale fetched
  outline, debounced by 120 ms so a drag does not compete with the pointer for
  the frame budget. Markers stay visible mid-drag; repairs stay disabled,
  because the kernel has not accepted that geometry yet.
- A fourth check reports a curve extreme that no node sits on, with a 0.5-unit
  margin so a rounding artefact is not marked.
- Every issue now carries the repair it would apply, so Fix appears exactly when
  there is one: an open contour is closed by joining its two ends, an adjacent
  run of duplicate nodes is deleted keeping the shape, a fractional point is
  rounded, and a missing extreme gets a node. A coincidence spanning two
  contours or involving a handle is reported for review without a fix, because
  it is ordinary drawing.
- Repairs re-run the audit over freshly fetched contours and apply only issues
  whose repair is still exactly the one the user reviewed.
- Because a structural repair renumbers its contour's points — and inserting a
  node re-parameterises the segment it splits — Fix All applies one kind of
  structural change per contour **and one extreme per segment**, and says so.
  Contours and segments are otherwise independent, so a glyph with issues spread
  across them is still repaired in one undo entry. The repairs are emitted in a
  total order (kind, then descending segment) so no command in a batch names an
  index an earlier one has renumbered.
- During a gesture the audit refreshes at least every 250 ms as well as 120 ms
  after the outline settles, so a drag that never pauses still moves its
  markers instead of freezing them on the gesture's first frame.
- The per-check switches persist per viewer in `localStorage`, with every read
  and write guarded so blocked site data falls back to every check on.

### Row 81 — the marker size reaches hit testing

The hit radius of a node and of a handle is now the larger of its share of the
hit distance and half of the marker actually painted, clamped to the range the
preferences accept. Raising *Node size* or *Handle size* therefore makes those
points easier to grab as well as easier to see, which is what "applied to
drawing and hit testing consistently" has to mean.

### Row 33 — components placed by their anchors

`Component.auto_align` and `Component.anchor` were stored, round-tripped,
honoured as a drag lock and used by metrics inheritance, but nothing ever
derived a component's position from an anchor pair: the flag claimed an
alignment the code did not perform. `Command::AlignComponents` now solves the
translation of every auto-aligned component of a layer so that the base layer's
mark anchor — `Component.anchor` when it names one, otherwise the first
`_`-prefixed anchor — lands on the base attachment anchor of the same name. The
attachment anchor is looked for in the carrier's own anchors first and then in
the base glyphs of the components before it, nearest first, each transformed by
its own component transform, which is the order a composite is read in. Only the
translation is rewritten: scale, rotation and skew stay as drawn, and a
component whose pair does not resolve is left where it is rather than moved
somewhere arbitrary. Resolution never recurses into a component's own
components, so a cyclic reference cannot loop.

A **locked** component is skipped rather than moved: preserving it is exactly
what its lock promises, and one locked mark does not stop the rest of the layer
being placed. The Glyph menu entry is enabled on the same rule — a layer whose
only auto-aligned component is locked has nothing to align. Placement is
**progressive**, so a composite attaching to an earlier auto-aligned component
reads where that component has just been put rather than where it used to be.

The user reaches it three ways: turning **Auto-align** on in the Components
panel places the component in the same undo entry, **Glyph → Align Components to
Anchors** places every auto-aligned component of the layer, and — since the
review — **moving a base glyph's anchor re-places its dependents automatically**.
That last one is a post-command pass in `crates/fc-doc/src/command/apply.rs`:
the layers a moved anchor can reach are collected *before* the command runs and
added to `Impact::glyphs`, so they are inside the undo snapshot and the whole
cascade is one undo entry. The cascade is bounded to four rounds, which covers
`a` → `aacute` → `aacutegrave` and cannot spin on a cyclic component graph.

### Row 89 — a visible reason for a refused edit

A locked glyph and locked paths already refused edits in the kernel and greyed
their controls out. The Paths and annotations panel now also states the reason
in a live region, and the disabled controls carry it as a tooltip.

A press on the canvas was still silent, because the locked geometry is taken out
of the hit test before the click is resolved and a refused press is
indistinguishable from a press on empty canvas. `EditorGeometry.refusedAt` asks
the second question — "was there something there you are not allowed to move?" —
over the unfiltered outline, and the canvas answers with one toast naming either
the path lock or the glyph lock. Repeats collapse into the toast already
showing, and genuinely empty canvas stays quiet.

### Requested by task d — the beam's pointer hooks

Task `p5f-d` asked for three pointer hooks in `GlyphCanvas.tsx` so the Spacing
tab's measurement beam can be grabbed on the canvas instead of only through its
slider. `TextCanvasRun` gained three optional callbacks (`onBeamGrab`,
`onBeamDrag`, `onBeamRelease`), and the canvas now offers a press to the beam
after the gap handles and before the glyph under the pointer, within a 5 CSS
pixel band. The decision is a small state machine (`beamPointer.ts`) rather than
inline canvas code, so "a grab is always followed by a release", "a move during
a beam drag never reaches the editor controller" and "a run that disappears
mid-drag ends the gesture" are all asserted. Task `d` owns what a new height
does; this task owns where the line is and what a pointer position measures.

## Verification

### Rust

Every cargo command ran under the wave's shared lock with a task-owned
`TMPDIR`, as `TMPDIR=.scratch/p5f-c-editor/tmp` below abbreviates.

```text
TMPDIR=.scratch/p5f-c-editor/tmp flock .scratch/p5f/cargo.lock \
  cargo test -p fc-geom --test corner --test extrema --test harmonize \
    --test direction --test smart_corners
  corner 8 passed · direction 5 passed · extrema 5 passed · harmonize 5 passed
  smart_corners 8 passed — 31 passed, 0 failed

TMPDIR=.scratch/p5f-c-editor/tmp flock .scratch/p5f/cargo.lock cargo test -p fc-model --test serde_roundtrip
  6 passed, 0 failed

TMPDIR=.scratch/p5f-c-editor/tmp flock .scratch/p5f/cargo.lock \
  cargo test -p fc-doc --test path_operations --test outline_commands \
    --test delete_points --test layers --test layer_workflows \
    --test editor_metadata --test component_locks --test problems \
    --test super_smooth --test fcf_roundtrip
  component_locks 10 · delete_points 9 · editor_metadata 10 · fcf_roundtrip 6
  layer_workflows 7 · layers 3 · outline_commands 16 · path_operations 7
  problems 8 · super_smooth 4 — 80 passed, 0 failed

TMPDIR=.scratch/p5f-c-editor/tmp flock .scratch/p5f/cargo.lock cargo test -p fc-api --test dispatch
  11 passed, 0 failed

TMPDIR=.scratch/p5f-c-editor/tmp flock .scratch/p5f/cargo.lock \
  cargo clippy -p fc-geom -p fc-model --all-targets -- -D warnings
  Finished, no warnings

TMPDIR=.scratch/p5f-c-editor/tmp flock .scratch/p5f/cargo.lock cargo clippy -p fc-doc --lib --tests -- -D warnings
  Finished, no warnings

TMPDIR=.scratch/p5f-c-editor/tmp flock .scratch/p5f/cargo.lock cargo clippy -p fc-api --all-targets -- -D warnings
  Finished, no warnings

TMPDIR=.scratch/p5f-c-editor/tmp flock .scratch/p5f/cargo.lock cargo check -p fc-formats -p fc-compile --lib
  Finished
```

The two `fc-api` dispatch failures task `b` reported are this task's:
`dispatch.rs` compares the whole `prefs.get` payload literally, so the nine new
per-target distances had to be added to both literals. `clippy -p fc-api
--all-targets -- -D warnings` is clean on this checkout; the `manual_contains`
hunk reported at `handlers/glyphs.rs:276` is already a `contains` call in the
working tree and no longer reproduces.

Three of the eight `fc-geom` corner tests failed first, for the reasons
test-first work produces: a fixture compared against the wrong baseline, a
"parallel tangents" case whose fixture was not parallel, and a seam case that
described the wrong pair of nodes. A fourth revealed a real sign error in the
reconnect parameters and a missing degenerate-result guard, both of which the
implementation now handles.

This also closes every "pending central Rust run" note left by the earlier
batch: the targets named as unexecuted in `p5-outline-node-semantics-acceptance.md`,
`p5-path-operations-acceptance.md`, `p5-layer-command-workflows-acceptance.md`
and `p5-89-editor-metadata-backend-acceptance.md` all pass on this checkout.

### Frontend

The owned directories, run with one worker and a task-owned `TMPDIR`:

```text
TMPDIR=.scratch/p5f-c-editor/tmp pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/editor src/views/glyph src/stores src/dialogs/PreferencesDialog.test.tsx src/shell
  Test Files  134 passed (134)
  Tests       1238 passed (1238)
  Duration    88.02s
```

The whole application suite was run too, and the only failure in it is another
task's: `src/i18n/i18n.test.ts` wants a zh-CN counterpart for
`fontInfo.scoped.derived`, which is not a key this task added (every key it adds
has one).

```text
TMPDIR=.scratch/p5f-c-editor/tmp pnpm --dir app exec vitest run
  Test Files  1 failed | 311 passed (312)
  Tests       1 failed | 2506 passed (2507)
```

That run includes the tests written for this task: eight geometry and document
corner cases, four for the Pixel handover, four for guide anchoring, four for
per-target snap reach, two for the marker-aware hit radius, eleven for the
extended audit plus three for its persisted switches, six for clearing a
background, five for the tool commands, seven for the beam pointer and six for
the font's own grid and increments (row 53, two of them rendering the hook). The
audit's component tests found a real consequence of persisting the switches —
one test turning a check off silently disabled the next test's fixes — which is
now reset between tests.

TypeScript, ESLint and Prettier for the owned files:

```text
pnpm --dir app check          # no error in src/editor, src/views/glyph,
                              # src/stores/prefsStore.ts or src/dialogs
pnpm --dir app exec eslint src/editor src/views/glyph \
  src/dialogs/PreferencesDialog.tsx src/stores/prefsStore.ts
  0 errors (one pre-existing react-hooks warning in GlyphVariationPanel.tsx,
  which belongs to task j)
pnpm --dir app exec prettier --write <owned files>
pnpm --dir app exec tsc -p tsconfig.node.json --noEmit   # the e2e specs
```

`pnpm --dir app check` also reports errors outside this task's files — other
`p5f-*` areas are mid-change in the same checkout. The count above is the
filtered result for the owned directories; the integrator owns the whole-app
number.

### Browser

A development kernel on 8803 with `fixtures/fonts/DejaVuSans.ttf` and Vite on
1503, driven headlessly. Every assertion below is what the kernel stored, read
back through `glyphs.outline` and `prefs.get`, not what the canvas drew.

- **Row 27.** Glyph `A` opened from the grid; the apex node selected with a real
  pointer press; **Path → Open Corner** takes contour 1 from 8 to 9 points, and
  `Ctrl+Alt+O` does the same. A marquee around the mouth selects exactly its two
  nodes ("2 points selected"), and **Path → Reconnect Nodes** returns the
  contour to its original eight points at their original coordinates
  `(586, 1493) (815, 1493) (1384, 0) …`. Two gestures, two undo entries: one
  `Ctrl+Z` restores the open corner, a second restores the glyph exactly.
  Screenshots: [the Path menu](screenshots/p5f-c-editor/path-menu.png),
  [the opened corner](screenshots/p5f-c-editor/open-corner.png).
- **Row 20.** The strip lists all sixteen entries including **Pixel**, and
  choosing Pixel opens the Bitmap tab and leaves the Edit tab on its previous
  tool (Playwright). Screenshots: [the tool strip](screenshots/p5f-c-editor/tool-palette.png)
  and [the same tool in the command palette](screenshots/p5f-c-editor/tool-palette-command.png).
- **Row 30.** Adding a vertical layer guide stores `x 701`; choosing *Right
  sidebearing* stores `{"x": -683, "anchor": "Rsb"}` while the line stays at
  701 — the ink ends at 1384, and 1384 − 683 = 701. Screenshot:
  [the guide's anchoring](screenshots/p5f-c-editor/guide-anchor.png).
- **Row 31.** Preferences → Snapping writes `snap.distances.nodes = 18` and
  reads it back from the kernel; the other eight stay at zero, which means the
  shared distance. Screenshot:
  [per-target distances](screenshots/p5f-c-editor/snap-distances.png).
- **Row 35.** After the corner is opened, the audit reports its two fractional
  points live, each with a Fix, and **Fix All** rounds both: every stored
  coordinate of the glyph is an integer again, in one undo entry. Screenshot:
  [the audit](screenshots/p5f-c-editor/outline-audit.png).

### Shared files touched

Everything else is inside this task's ownership list. These are the narrow,
additive patches to files it shares or does not own, each named so the owning
task can see it:

| File                                            | Owner  | Change                                                                             |
| ----------------------------------------------- | ------ | ---------------------------------------------------------------------------------- |
| `crates/fc-doc/src/command.rs`                   | none   | Three new variants: `OpenCorner`, `ReconnectNodes`, `AlignComponents`.             |
| `crates/fc-doc/src/command/apply.rs`             | none   | Routing and impact for those three.                                                |
| `crates/fc-api/src/types/session.rs`             | `p5f-a`| `SnapPrefs.distances` (`SnapDistances`, all defaulting to zero); `Eq` dropped from `SnapPrefs` because it now holds `f64`. |
| `crates/fc-api/src/handlers/prefs.rs`            | `p5f-a`| Validates the nine new distances as finite and within 0–64.                        |
| `crates/fc-api/tests/dispatch.rs`                | `p5f-a`| The two literal `prefs.get` payloads gained the `distances` object, which the new field adds to the serialised preferences. |
| `crates/fc-model/src/lib.rs`                     | shared | Exports `GuideAnchor`.                                                             |
| `crates/fc-formats/src/{ufo/info.rs,glyphs/outlines.rs}` | `p5f-f` | One `..Guide::default()` each, so the new field does not break their literals. |
| `crates/fc-model/tests/serde_roundtrip.rs`       | shared | The master guide fixture now also round-trips an anchor.                            |
| `crates/fc-geom/src/lib.rs`                      | shared | Exports `open_corner`, `reconnect_nodes`, `CornerError`.                            |
| `crates/fc-doc/src/fcf/schema.md`                | shared | `Guide.anchor`, the corner commands and `AlignComponents`.                          |
| `app/src/stores/prefsStore.ts`                   | shared | Reads the per-target distances and the marker sizes into `SnapSettings`.            |
| `app/src/dialogs/PreferencesDialog.tsx`          | `p5f-m`| A distance field beside each snap switch.                                           |
| `app/src/dialogs/dialogs.module.css`             | shared | One new `.snapRow` grid for those rows.                                             |
| `app/src/shell/usePaletteEntries.ts`             | `p5f-a`| A palette-only command shows its own detail; a command with a menu is unchanged.    |
| `app/src/views/text/textCanvas.ts`               | `p5f-d`| The three optional beam callbacks task `d` asked for.                               |
| `app/src/i18n/en.json`                           | shared | New keys under `tool`, `editor.audit`, `editor.undo`, `cmd.path`, `cmd.glyph`, `cmd.layer`, `cmd.reason`, `panel.layers`, `infoBox`, `editorMetadata` and `dialog.prefs`. |

`app/src/views/glyph/EditGlyphList.tsx` also gained the `referenceCharacters`
prop that task `p5f-b` added to `GridCells`; it is this task's file, but the
requirement came from theirs.

## Per-row recommended status

| Row | Recommended        | Evidence and boundary                                                                                                                                                            |
| --- | ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 20  | **Complete**       | Sixteen strip entries, each selectable from the strip, by its key and by a `tool.<id>` palette command; Pixel opens the strike editor and restores the previous tool.             |
| 21  | **Complete**       | Tap/hold with live keymap overrides; 41 shell and glyph-view shortcut tests, plus the strip's borrowed state.                                                                    |
| 22  | **Complete**       | Double-click / Return and the Alt outline-preserving variant, with the kernel's `SetSmooth` behind them.                                                                          |
| 23  | **Complete**       | G2 nodes and Harmonize in geometry, document and UI; general *quadratic* G2 remains out of scope, as the earlier record states.                                                   |
| 24  | **Complete**       | Physical Ctrl separated from the platform modifier, Ctrl suspends snapping, 1/10/100 nudges. Native macOS and Windows input cannot be exercised here.                             |
| 25  | **Complete**       | Proportional handle nudge during a drag and in arrow runs.                                                                                                                       |
| 26  | **Complete**       | Shift-click segment insertion, Add Extremes and Force Extremes, the last with its own geometry and document tests.                                                                |
| 27  | **Complete**       | Tidy, direction, both all-masters variants, reverse, break, join, **Open Corner** and **Reconnect Nodes**, driven from the Path menu against the real kernel. Convex limit below. |
| 28  | **Complete**       | X/Y/W/H, nine-point origin, proportional lock, Tab traversal and arrow increments; the guide rows now also carry anchoring.                                                       |
| 29  | **Complete**       | Cursor HUD with delta, angle, length, marquee size, stem width and the snapped target's name.                                                                                    |
| 30  | **Complete**       | Local and global guides, angle, lock, naming, measurement spans and **LSB/RSB anchoring**, verified against the stored guide. Measurement is view state, not stored data.        |
| 31  | **Complete**       | Nine independent targets, **one distance each**, zones, background, component ghosts, grid subdivision, Ctrl suspension.                                                          |
| 32  | **Complete**       | Show, `Mod+B` edit, selection→background, copy, swap, **clear**, per master.                                                                                                     |
| 33  | **Complete**       | Decompose, lock and anchor alignment all work; alignment runs when it is switched on, on demand, and now whenever a base glyph's anchors move — one undo entry each. Locked components are preserved, chained composites follow. |
| 34  | **Complete**       | Mark-cloud preview over the paginated Mark inventory.                                                                                                                            |
| 35  | **Complete**       | Live audit including mid-gesture with a 250 ms ceiling, four checks, four repairs, persisted switches, Fix and Fix All against the real kernel, one structural kind per contour and one extreme per segment. |
| 36  | **Complete**       | Masters, brace/bracket, backgrounds, backups and custom layers with add, duplicate, rename, remove and colour.                                                                   |
| 81  | **Complete**       | DPR canvas, CSS-pixel hit testing, configurable hit distance **and** marker sizes that reach hit testing.                                                                        |
| 89  | **Complete**       | Path focus, path and glyph locks, lock-aware tools, and a visible reason for a refused edit.                                                                                     |
| 90  | **Complete**       | Measure tool with rulers, intersection spans, italic-aware deltas and `G` to drop a guide mid-drag.                                                                              |
| 91  | **Complete**       | Six annotation kinds stored per layer, edited from the inspector, one command per change.                                                                                        |

### Playwright

`app/e2e/draw.spec.ts` gains an Open Corner / Reconnect Nodes spec and
`app/e2e/glyphview.spec.ts` three specs for the Pixel handover, guide anchoring
and the per-target snap distance, all against the real kernel on 8803/1503. The
per-test budgets are `test.slow()` rather than fixed numbers, so the suite's own
`--timeout` still governs them.

The three `glyphview` specs pass:

```text
TMPDIR=.scratch/p5f-c-editor/tmp FC_API_PORT=8803 FC_UI_PORT=1503 \
  pnpm --dir app exec playwright test e2e/glyphview.spec.ts \
  --grep "editor tools" --workers=1 --reporter=line --timeout 60000
  3 passed (2.5m)
```

Both `draw` specs pass:

```text
TMPDIR=.scratch/p5f-c-editor/tmp FC_API_PORT=8803 FC_UI_PORT=1503 \
  pnpm --dir app exec playwright test e2e/draw.spec.ts --reporter=line
  2 passed (9.9s)
```

Getting there took four distinct fixes, and the first diagnosis in this file was
wrong, so the record is corrected here rather than quietly replaced.

1. **The tool key was never lost.** The earlier note in this document claimed
   `p` disappeared between Playwright's `keyboard.press` and
   `useToolShortcuts`. It did not. At 1280x720 the Edit tab's text bar, spacing
   and kerning chrome squeezed the canvas — a flex item with `min-height: 0` —
   down to about 140 px, and to nothing at all at 1280x577. The click meant to
   focus the canvas therefore landed in the chrome, `useToolShortcuts` correctly
   ignores keys typed in a field, and the key did nothing. The fix is on both
   sides: `glyphView.module.css` gives the stage a 240 px floor and the view a
   260 px one, and both specs click the canvas centre and then assert the tool
   strip shows the new tool pressed, so a repeat of this fails on the tool, not
   twenty lines later on an empty outline.
2. **The Info box floats over the canvas's lower left**, and on a canvas at its
   floor it covers most of the left half. Clicks there are swallowed. The Open
   Corner spec now measures the Info box and places the whole chevron in the
   clear region to its right; every canvas click also carries a 15 s timeout, so
   a covered point is reported as an interception instead of consuming the
   test's entire budget in silence.
3. **`getByLabel('X')` was ambiguous**: it also matches the "Collapse info box"
   button. The assertion is now by role with an exact name.
4. **The Recover Documents modal.** Playwright's call log named it exactly:
   `<div class="_overlay_mmejc_3"> intercepts pointer events`. Earlier
   interrupted runs left `Draw ...` and `Corner ...` documents dirty in a
   development server that `reuseExistingServer` then reused, and its autosave
   produced recovery entries on the next launch. Both specs now dismiss the
   prompt with "Later" after loading. This is worth keeping: an interrupted
   browser run leaves a dirty document behind, and the next run starts inside a
   modal.

Both specs also select the new document's tab explicitly and assert
`aria-selected` rather than assuming that creating a font focuses it.

The layout floor closes the glyph-view half of the squeeze. The other half is
not this task's file: once the chrome no longer fits above a 240 px canvas, the
column has to scroll or collapse, which is `.view` in
`app/src/views/text/TextView.module.css` (task `d`): its `.view` is the flex
column that stacks `TextWorkspaceControls`, `GlyphView` and `PreviewStrip`, and
it carries `min-height: 0` with no overflow rule. The exact change needed there
is `overflow-y: auto` on that `.view`, so the chrome gives way instead of the
canvas once the three of them no longer fit.

Machine load has to be stated too: eight `p5f-*` agents shared this checkout,
the load average sat between 12 and 38, and opening one glyph tab on DejaVuSans
triggers a preview compile of 6,253 glyphs. Per-test budgets are `test.slow()`
so a quiet machine's own `--timeout` governs them.

## Review round

The review returned ACCEPT WITH FIXES. Each item, and the test that holds it:

| # | Item | Fixed in | Proven by |
| - | ---- | -------- | --------- |
| 1 | Fix All inserted a node off the extremum when one segment carried two | `app/src/views/glyph/outlineAudit.ts` — one insertion per `(contour, segment)` | `outlineAudit.test.ts` › *two extremes on one segment* › "reports both but repairs only one per pass" (the reviewer's `(0,0) (300,400) (-300,400) (0,800)` cubic, extremes at t ≈ 0.2113 and 0.7887) and "still repairs one extreme per segment across several segments" |
| 2 | `AlignComponents` moved locked components | `component_locks.rs` skips a locked component instead of failing the command; `layerCommands.ts` enables the menu entry only for an **unlocked** auto-aligned component | `fc-doc/tests/component_locks.rs` › `a_locked_component_is_left_where_it_is_and_does_not_block_the_others`; `layerCommands.test.ts` › "is offered only when a component asks to be aligned" |
| 3 | Chained composites aligned to a stale transform | `component_locks.rs` places into a progressive `working` copy | `a_chained_composite_follows_the_component_it_attaches_to` |
| 4 | No re-alignment when a base glyph's anchors move | `apply.rs::realign_after_anchor_edit` plus `auto_align_closure`, whose layers join `Impact::glyphs` **before** the command runs so the cascade is one undo entry | `moving_a_base_anchor_moves_its_dependents_in_the_same_undo_entry` (asserts undo and redo restore both glyphs) |
| 5 | Comparator was not a total order; `ALL_OUTLINE_AUDIT_TESTS` aliased the default object; the debounce had no ceiling; a press on a locked path was silent | `outlineAudit.ts` (kind, then descending segment, then descending t; `{ ...DEFAULT_OUTLINE_AUDIT }`), `OutlineAudit.tsx` (`AUDIT_MAX_WAIT_MS = 250`), `editorGeometry.ts`/`editorController.ts` (`refusedAt` → `onRefused`) | "still repairs one extreme per segment across several segments"; `OutlineAudit.test.tsx` › "publishes on the ceiling even while every frame restarts the debounce"; `editorController.test.ts` › "reports the lock when a press lands on a locked path", "names the glyph lock instead when the whole glyph is locked", and the case where empty canvas stays quiet |
| 6 | The modifier grammar was not documented | `docs/user/outline-editor.md` — a table of every gesture modifier, and the note that `Mod` is Ctrl on Linux and Windows and Command on macOS while `Ctrl` always means the physical key, so on Linux Ctrl is both the ×100 increment and the snap suspension | The document itself; `modifiersOf` keeps the two distinct on macOS (limit 5 below) |
| 7 | The `draw` specs failed | Canvas height floor, centre click, Info-box-aware positions, exact locator, recovery dismissal | `pnpm --dir app exec playwright test e2e/draw.spec.ts` — 2 passed |

## Remaining limits

1. **A convex open corner** leaves an ear after overlap removal, as described
   above and asserted by name in the geometry tests.
2. **The measurement-guide switch is view state**, not stored data: a guide
   marked as a measurement guide loses that mark when the tab closes. Position,
   angle, name, lock and anchoring all persist.
3. **General quadratic G2** is still not provided; the G2 work is
   cubic-to-cubic, as `p5-outline-node-semantics-acceptance.md` records.
4. **Interchange formats carry no guide anchoring.** UFO, Glyphs and SFD have no
   equivalent field, so a guide exported and reimported comes back as `Origin`.
   The native `.fcf` keeps it.
5. **Native macOS and Windows input** cannot be exercised here. The modifier
   grammar is verified on Linux, where `Mod` is Ctrl; the platform mapping is
   explicit in `modifiersOf`, so Command and physical Control stay distinct on
   macOS, but no macOS or Windows run backs that.
6. **Fix All needs a second press** when one contour holds two different kinds
   of structural defect, or when one segment is missing two extremes: inserting
   a node re-parameterises the segment it splits, so the second parameter would
   name a curve that no longer exists. The remaining issues stay listed rather
   than being silently dropped.
7. **The anchor cascade is bounded to four rounds.** Moving a base glyph's
   anchor now re-places its dependents in the same undo entry, which covers
   `a` → `aacute` → `aacutegrave`; a chain deeper than that needs one
   *Glyph → Align Components to Anchors*. The bound is what makes a cyclic
   component graph impossible to spin on.
