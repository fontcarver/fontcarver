# P5f-a acceptance — shell, windows, workspaces and project file

Area `a-shell`, audit rows 1–9. Task file:
[`p5f-a-shell.md`](superpowers/plans/tasks/p5f-a-shell.md), whose
`## Gap analysis` section records what each row was missing at the start and the
decision taken for it. Protocol:
[`p5f-protocol.md`](superpowers/plans/tasks/p5f-protocol.md). Ports API 8801,
UI 1501. Platform: Linux, `fc-devserver` + Vite, Chromium through Playwright.

Every claim below names the command or browser step that produced it.
Everything still unverified is under **Limits**, not omitted.

## What changed

- **Pinned Overview and ordinals (row 1).** `uiStore.openTab` keeps each
  document's tabs in one contiguous run with its Overview first, so the strip,
  `Ctrl+Tab` and the numbered shortcuts agree on what "the second tab of this
  document" means. `Mod+Alt+2…9` now address every editing tab of the active
  document in strip order; previously they counted Glyph tabs only, so a Font
  Info, Features or Bitmap tab was unreachable by ordinal. `Close Tab` is
  refused on the pinned Overview and says to use **File → Close Font**. Each tab
  button carries `data-pinned` and `aria-keyshortcuts`, so its binding is
  announced rather than only documented.
- **Tear-off (row 2).** The preview strip gained `window.detachPreview`, so
  every detachable view has a menu and palette route and not only a pointer
  target. `uiStore` records which views are in their own windows (`detached`),
  cleared when a window returns or its document closes.
- **Preview strip (row 4).** `view.textPreview` gained `Mod+3`, beside `Mod+1`
  and `Mod+2` for the two docks, and `view.previewCollapse` (`Mod+Alt+P`)
  collapses and expands the strip of the active Edit tab.
- **Palette sections (row 5).** Results are grouped into labelled
  `role="group"` sections in the documented order (commands, filters, scripts,
  glyphs) while keeping one flat keyboard traversal order; the per-row category
  badge was dropped as redundant with the section label.
- **Workspaces in the project (row 6).** New kernel routes
  `session.workspaces` / `session.set_workspaces` keep a document's named
  workspaces in a deterministic sidecar — `workspaces.json` inside a `.fcfp`
  project, `<name>.fcf.workspaces.json` beside a single-file document — so a
  saved arrangement travels with the document through Git.
  `stores/workspaceProject.ts` merges the stored records into the existing store
  when a document opens and writes them back when one is saved or deleted.
  Detached windows became part of the snapshot, stored by panel or by glyph
  _name_, and applying an arrangement reopens them.
- **History with scope labels (row 8).** `UndoEntry::scope()` derives a scope
  from the snapshots an entry already holds: the glyphs, the layer names obtained
  by diffing the entry's before/after glyph values, the bitmap strikes, and
  whether font-wide data is involved. It is computed when the History surface
  asks, so the editing path pays nothing. `doc.history` lists the timeline and
  `doc.history_go` moves the cursor to an exact position, rejecting a request
  built from a stale length. A **History** section in the Inspector lists one row
  per gesture with its scope, marks the current position, keeps undone entries
  listed so redo is reachable from the list, and has **Edit → History**.
- **Determinism and recovery (row 7).** New tests hold `.fcfp` output to "a
  re-save of an unchanged document changes no byte on disk", and follow an
  interrupted project session through to a recovered document.
- **Platform parity (row 9).** A parity test holds the single command table to
  the `Mod` abstraction and checks that the same keystroke resolves to the same
  action with Command on macOS and Control elsewhere.
- **Desktop binary command line and headless failure (from the release task).**
  `fontcarver-desktop` now accepts `--version`/`-V` and `--help`/`-h`, refuses
  any other argument with the usage text and exit code 2, and turns a missing or
  unreachable graphical session into one readable line on stderr and exit code 1
  instead of a GTK panic and exit 101. The panic tao raises inside Tauri is
  caught, its payload — GTK's own wording — is reported, and the default panic
  hook's backtrace is suppressed for that step.
- **Bundle identifier (from the release task).** `src-tauri/tauri.conf.json`
  now uses `com.fontcarver.desktop`; `com.fontcarver.app` ended in `.app`, which
  collides with the macOS bundle extension and made the Tauri CLI warn on every
  build.

## Verification

### Rust

```text
cargo test -p fc-doc --lib undo::                  7 passed
cargo test -p fc-doc --test directory_project      9 passed
cargo test -p fc-doc --test history_recovery       6 passed
cargo test -p fc-api --lib workspaces::            7 passed
cargo test -p fc-api --lib                       225 passed
cargo test -p fontcarver                          15 passed (10 lib + 5 binary)
                                                  (run twice, the second after
                                                  the final change; a third
                                                  confirmation run was stopped
                                                  while queued on the shared
                                                  Cargo lock and is not counted)
cargo check -p fontcarver                        Finished `dev` profile
cargo clippy -p fc-doc     --all-targets -- -D warnings   clean
cargo clippy -p fc-api     --all-targets -- -D warnings   clean
cargo clippy -p fontcarver --all-targets -- -D warnings   clean
rustfmt --edition 2021 <changed files> ; git diff --check  clean
```

New Rust tests, each red before its implementation existed:

- `undo::scope::tests` — a glyph edit names the glyph and only the layer that
  actually changed; a created or deleted glyph reports no layer noise; an added
  layer is named from the new value; font-wide snapshots are font level; a bitmap
  edit reports its glyph and strike; an anchor edit names its layer with no glyph
  diff; an entry whose snapshots were shared away has an empty scope.
- `handlers::workspaces::tests` — sidecar location for `.fcfp`, `.fcf` and
  `.fcf.gz`; no sidecar for `.ttf`, `.ufo` or an extensionless path; a saved list
  round-trips byte-identically with a trailing newline; a missing sidecar reads
  as an empty list; corrupt, foreign-format, future-version and out-of-bounds
  sidecars are reported rather than ignored; invalid lists are rejected before
  anything is written; an oversized list is refused.
- `directory_project::resaving_an_unchanged_project_produces_no_diff` — three
  successive saves of an unchanged document (including one through a fresh
  `Session`) produce byte-identical files and names, every text file ends with a
  newline, and one glyph edit adds one glyph revision, prunes the
  old one and rewrites the manifest, leaving the file count unchanged.
- `autosave::tests::an_interrupted_project_session_is_offered_for_recovery_and_restores_its_work`
  — a `.fcfp` project edited without saving, then "crashed": the snapshot lands
  in the recovery directory pointing at the project, the committed project still
  holds the last saved state, recovering reopens the unsaved work, saving it
  overwrites the project, and it is then no longer offered.
- `src-tauri/src/cli.rs` and `src-tauri/src/error.rs` unit tests — argument
  parsing in both spellings, refusal of anything else with the usage text, a
  usage exit code distinct from a startup failure, and the missing-display
  message naming both the cause and the command-line alternative.
- `src-tauri/tests/cli.rs`, which runs the shipped binary — `--version` prints
  `fontcarver-desktop <version>` and exits 0; `--help`/`-h` print the usage and
  exit 0; an unrecognized argument exits 2 with the usage text; with neither
  `DISPLAY` nor `WAYLAND_DISPLAY` the binary exits 1 saying no graphical session
  is available; with `DISPLAY=:987 GDK_BACKEND=x11` it exits 1 (never 101) with
  GTK's own message and no backtrace. Observed directly:

```text
$ target/debug/fontcarver-desktop --version
fontcarver-desktop 0.1.0                                          exit 0
$ target/debug/fontcarver-desktop --porcelain
fontcarver-desktop: unrecognized argument `--porcelain` + usage      exit 2
$ env -u DISPLAY -u WAYLAND_DISPLAY target/debug/fontcarver-desktop
fontcarver-desktop: no graphical session is available (neither DISPLAY
nor WAYLAND_DISPLAY is set). …                                       exit 1
$ DISPLAY=:987 GDK_BACKEND=x11 target/debug/fontcarver-desktop
fontcarver-desktop: no graphical session is available (Failed to
initialize gtk backend!: BoolError { … }). …                         exit 1
```

### Frontend

```text
pnpm --dir app gen:api                            regenerated (4 new types, 4 new methods)
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/shell src/stores/{uiStore,workspaceStore,workspaceProject,sessionStore,
  sessionProjectPersistence,sessionTabBarriers}.test.ts \
  src/dialogs/{WorkspaceDialog,RecoveryDialog}.test.tsx src/App.test.tsx
  Test Files  36 passed (36)
       Tests  296 passed (296)
pnpm --dir app exec eslint src/shell src/stores/uiStore.ts \
  src/stores/workspaceSnapshot.ts src/stores/workspaceProject.ts \
  src/stores/workspaceProject.test.ts src/dialogs/WorkspaceDialog.tsx
  exit 0, no diagnostics
pnpm --dir app exec prettier --write <changed files>   all formatted
pnpm --dir app check    no diagnostics in this task's files
```

`pnpm check` does report diagnostics in `src/views/font/GlyphCell.test.tsx`,
`src/views/glyph/panels/*`, `src/views/text/{SpacingMatrix,spacingMatrix,
useSpacingMatrix}*`, `src/dialogs/Export*`, `src/stores/exportProfileStore.test.ts`
and `src/plugins/*`. Those files belong to other P5f areas and were mid-edit
throughout this task; none of them is in this task's ownership list. Two
diagnostics that _were_ this task's — the palette's `openScript` call and its
route test's script fixtures — were fixed here after `gen:api` added the new
`folder` field to `AutomationScriptEntry`.

New frontend tests:

- `stores/uiStore.test.ts` — the pinned Overview is inserted before its
  document's other tabs and a document's tabs stay contiguous; a detached view is
  recorded once and forgotten when it returns; a closed document takes its
  detached windows with it.
- `shell/workspaceTabCommands.test.ts` — ordinals reach every editing tab and
  stop past the ninth; the Overview stays pinned first; `Mod+K`; the preview
  collapse command toggles the active Edit tab's strip and is unavailable with no
  Edit tab or a hidden strip.
- `shell/TabBar.test.tsx` — the pinned marker, title and each tab's
  `aria-keyshortcuts`.
- `shell/windowActions.test.ts` — every detached view is recorded; reopening a
  saved list reports only the windows a browser refused.
- `shell/workspaceLifecycle.test.ts` — a restored arrangement reopens the
  windows it had detached, with the expected view URLs.
- `stores/workspaceStore.test.ts` — glyph windows are stored by name and
  reopened under new handles; a window whose glyph is gone is dropped rather
  than opened on another glyph; stored lists are validated and records saved
  before P5 (with no list) still load.
- `stores/workspaceProject.test.ts` — a project document's stored workspaces are
  loaded into the store; saving and deleting write back; an arrangement saved
  before the document had a project joins it; a document without native storage
  stays local and is never written; an unreadable record is reported once and
  the local workspaces are kept.
- `shell/HistoryPanel.test.tsx` — each gesture is listed with its scope and the
  current position is marked; selecting an entry states the length it was built
  from; the first row returns to the state before any edit; a stale list says so
  and reloads without moving the document; an empty timeline says nothing has
  been edited; the timeline is read only while the section is open.
- `shell/shellPlatformParity.test.ts` — every action is defined once with one
  platform-independent binding and no `Cmd`/`Meta`/`Win` literal; `Mod` resolves
  to Command on macOS and Control on Windows and Linux; every bound command
  dispatches to the same action from the Command form and the Control form.

### Browser (Chromium, API 8801, UI 1501)

`app/e2e/app.spec.ts` and `app/e2e/panels.spec.ts`, run against
`fc-devserver --port 8801 --open fixtures/fonts/DejaVuSans.ttf` and Vite on 1501:

```text
FC_API_PORT=8801 FC_UI_PORT=1501 pnpm --dir app exec playwright test \
  app.spec.ts panels.spec.ts --workers=1 --reporter=list
  9 passed, twice; 10 passed after the second-document case was added; 12 passed
  (45.0s) after the review fixes added the detached-Edit-panel and reopen cases
```

What those ten steps actually exercise, all against the real kernel:

- the shell chrome renders (pre-existing case);
- row 1: three tabs with the Overview pinned first and no close control,
  `aria-keyshortcuts` on each, `Mod+Alt+1/2/3` selecting by ordinal including a
  Font Info tab, `Ctrl+Tab` and `Ctrl+Shift+Tab` cycling, and **Close Tab**
  disabled on the Overview with the pinned reason;
- row 2: the Inspector torn into a real popup window, the dock giving up its
  space, the view rendering in the popup against the same document, and
  **Return to document window** restoring the dock as its own window closes;
  separately, an Inspector torn off an **Edit** tab showing live glyph metrics
  and following the glyph the owning window moves to;
- row 5: `Mod+K`, a query matching several categories, labelled `Glyphs` and
  other sections inside the listbox, and selecting a glyph opening its Edit tab;
- row 8: a typed LSB commit appearing as one entry scoped `Glyphs: E`, marked
  current; selecting the last row returning the metric to its original value and
  disabling itself; the undone entry staying listed and reapplying when selected;
- row 4: dragging the divider growing the strip by more than 40 px, the keyboard
  handle changing `aria-valuenow`, and `Mod+Alt+P` collapsing and expanding it;
- row 3: the Inspector's numeric metrics, a Path-menu operation and the Anchors
  panel each producing one undo entry, then three undos restoring the layer
  (pre-existing case, re-run against the current shell); the sidebar following
  the active tab kind and the layer list adding a background layer;
- row 6: hiding the Inspector and the preview strip, saving that arrangement as
  "Canvas only", restoring the panels by hand, and the saved arrangement putting
  them back, then deleting it;
- opening a second document through the Open Font dialog while the first one is
  on a Glyph tab: the new document's Overview is selected and pinned, both
  documents keep their own run of tabs, and closing it leaves the session as the
  other specs expect.

Screenshots, captured with the headless browser against the same servers:

| File                                                                       | What it shows                                                                                       |
| -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| [`palette-sections.png`](screenshots/p5f-a-shell/palette-sections.png)     | `Mod+K` with labelled Commands / Filters / Glyphs sections (85 / 69 / 50 rows for the query `a`)    |
| [`history-panel.png`](screenshots/p5f-a-shell/history-panel.png)           | the Inspector's History section listing `Rename style · Font-level · CURRENT` and the opening state |
| [`inspector-detached.png`](screenshots/p5f-a-shell/inspector-detached.png) | the main window after the Inspector was torn into its own window (the dock is gone)                 |
| [`recovery-dialog.png`](screenshots/p5f-a-shell/recovery-dialog.png)       | the Recovery dialog after a killed session, naming the `.fcfp` project and the snapshot time        |
| [`recovered-document.png`](screenshots/p5f-a-shell/recovered-document.png) | the recovered document: pinned Overview `⊙Overview · Shell.fcfp` and the "Recovered from …" banner  |

Rows 6 and 7 were also driven directly against the kernel, reading the files it
wrote rather than trusting the API's answer:

```text
session.new + session.save …/Shell.fcfp   → .fontcarver.lock, glyphs/, project.json
session.workspaces                        → [] with path …/Shell.fcfp/workspaces.json
session.set_workspaces "Canvas only"      → workspaces.json, pretty-printed, final byte 0x0a
session.save (no path, unchanged)         → diff -r against a copy: no difference
session.close + session.open              → reopened as a new doc id; workspaces still ["Canvas only"]
session.save …/Single.fcf + set_workspaces→ Single.fcf.workspaces.json beside the file
session.set_workspaces []                 → the sidecar file is removed, not left empty
set_workspaces "  padded  "               → invalid_params: names must not begin or end with whitespace
set_workspaces layout [1,2]               → invalid_params: must carry a JSON object layout
autosave_seconds=2, edit, kill -9, restart→ session.list [] and one recoverable entry for the project;
                                            project.json still holds the last saved family name
```

The same specs were also run with Playwright's default two workers. Both
specs drive **one shared kernel document**, so two workers make them race: the
virtualised grid does not fill within the default timeout while the other worker
edits, and two specs editing the same glyph's LSB read each other's values. The
grid waits were made explicit (`toBeAttached` with a longer budget plus
`scrollIntoViewIfNeeded`) and this task's history case moved to its own glyph and
now restores it, which removed the state collision. After those fixes the
two-worker run was **8 passed, 1 failed**, the failure being a 30 s timeout on a
menu click in the pre-existing `panels.spec` case while the other worker was
editing — every step took three to eight times longer than serially. The timing
sensitivity is recorded as a limit below; serially the same nine pass.

### Independent checks of the on-disk formats

`.fcfp` determinism and the recovery flow are checked by the Rust integration
tests above, which read and compare the actual files rather than the writer's
intent. The workspace sidecar is checked byte-wise by
`handlers::workspaces::tests::a_saved_list_round_trips_byte_identically`.

No emitted font binaries are part of this area, so the FontTools/FreeType/
HarfBuzz checks the protocol asks for do not apply to these rows; the formats
this task writes are JSON, and they are verified by reading them back.

## Per-row recommended status

| Row | Recommended status                  | Scope of the claim and what remains                                                                                                                                                                                                                                                                                                                                                                                     |
| --- | ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Complete** on Linux               | Pinned, non-closable Overview first; `Mod+Alt+1–9` over every editing tab; `Ctrl+Tab`/`Ctrl+Shift+Tab`; per-document windows implemented for the Tauri shell (`src-tauri/src/windows.rs`, `default.json` capability `fc-*`) with `cargo check -p fontcarver` and its three routing tests passing, and emulated by named popups in the browser transport. A running macOS or Windows desktop is task n's.                |
| 2   | **Complete** on Linux               | Tab and panel tear-off with pointer, drag-out, menu and palette routes; the detached view shares the document; rejoin and a direct window close both return the content. Verified in a real popup window; native windows are the same transport path but were not executed.                                                                                                                                             |
| 3   | **Complete**                        | Contextual left sidebar and a scrolling Inspector section stack whose sections remember open/closed, now including History. Verified in the browser and by `App.test.tsx` and `EditInspector.test.tsx` (other tasks' files, re-run here).                                                                                                                                                                               |
| 4   | **Complete**                        | Pointer drag, keyboard handle with `aria-valuemin/max/now`, height persisted per tab and saved with the workspace, and both show/hide and collapse/expand as commands with shortcuts.                                                                                                                                                                                                                                   |
| 5   | **Complete**                        | `Mod+K`, scoped sections over commands, taxonomy and saved smart filters, local scripts and debounced glyph search; keyboard navigable; a glyph opens its Edit tab.                                                                                                                                                                                                                                                     |
| 6   | **Complete** for native documents   | Named workspaces (tabs, dock visibility and widths, Inspector sections, preview state and height, detached windows) saved in the project sidecar and restored per document. A document with no FontCarver storage yet keeps its arrangements per machine — stated in the dialog and in `docs/user/shell.md`, and a deliberate boundary, not a missing feature.                                                          |
| 7   | **Complete** with a stated location | Per-glyph text projects that are deterministic across re-saves and local per glyph edit; recovery snapshots in the documented recovery directory with a `document_path` pointer back to the project, offered on the next launch. The task's scope note allows this location instead of a sidecar inside the project; the reason (a snapshot that survives damage to any project member) is recorded in `fcf/schema.md`. |
| 8   | **Complete**                        | One entry per gesture, listed with glyph/layer/strike/font-level scope, current position marked, undone entries still listed, jump by selection with stale-request rejection, and an Edit-menu command. Layer names come from a real before/after diff, so only layers the entry changed are named.                                                                                                                     |
| 9   | **Partial** — Linux verified        | One keymap with a `Mod` abstraction, held to that shape by a test, and the whole shell exercised on Linux. macOS and Windows execution remains pending task n; no native or global input automation was used anywhere in this task.                                                                                                                                                                                     |

A request handed over from the metrics area is also addressed here: after a
second document is opened, its pinned Overview must be the selected tab, because
the shared helper `openPathThroughDialog` waits for exactly that.

- Opening a document into a shell that already shows another one **does** select
  the new document's Overview. Two tests now hold that: a store test
  (`sessionStore.test.ts`, "selects the new document Overview so the shell shows
  what was opened", with the first document left on a Glyph tab) and a browser
  test (`app.spec.ts`, "a second document → opens with its own Overview selected,
  beside the first document", which opens `NotoSans-Regular.ttf` through the Open
  Font dialog and checks `aria-selected` and `data-pinned`).
- The case that did **not** hold is a document with a _remembered arrangement_:
  restoring it also restored the tab it had been left on, so the newly opened
  document could land on, say, its Features tab. A suite that opens the same path
  twice inside one browser context hits this, and the helper then times out.
  `workspaceLifecycle` now restores the remembered tab **only when the document
  is the first one in the shell** — the relaunch case row 6 is about. A document
  opened while another is already on screen gets its remembered tabs, panels and
  windows back with its Overview selected, because the user asked to see that
  document. `workspaceLifecycle.test.ts` covers both directions.
- Still not selected, by design: when the kernel already holds several documents
  at start-up, `sessionStore.start()` adopts them all and activates the **first**.
  A helper must not assume the last-listed document is selected in that case.

Two findings handed over from the release area are also closed here:

| Finding                                                                  | Status       | Evidence                                                                                                                          |
| ------------------------------------------------------------------------ | ------------ | --------------------------------------------------------------------------------------------------------------------------------- |
| `fontcarver-desktop` had no `--version`/`--help` and exited 101 headless | **Complete** | `src-tauri/{src/cli.rs,src/error.rs,tests/cli.rs}`; 15 tests in `cargo test -p fontcarver`, plus the four observed runs above.    |
| Tauri `identifier` ended in `.app`                                       | **Complete** | `src-tauri/tauri.conf.json` now `com.fontcarver.desktop`; capabilities reference window labels, not the identifier, so unchanged. |

## Review fixes (2026-09-30, ACCEPT WITH FIXES)

Eleven review findings and one further report from the metrics area are fixed
here; each has a test that fails without the fix.

| #   | Finding                                                                                                                                                                                                             | Fix and test                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Blocking, row 5.** The palette rendered grouped sections but the highlight, Arrow keys and Enter walked the flat ranked list, so Enter could fire an entry the user never saw.                                    | `CommandPaletteDialog` now flattens the sections once (`ordered`) and uses that list for `activeIndex`, every row's index, the scroll-into-view effect and `run()`. Two cases in `CommandPaletteRoutes.test.tsx`: the Nth ArrowDown highlights the Nth _visible_ row with exactly one row selected, and Enter opens the glyph that was highlighted.                                                                                                                                                                                                                                                                                                                     |
| 2   | **Blocking, row 2.** An Inspector or sidebar torn off an Edit tab showed "Loading glyph…" for ever: the panels read `editorStore.activeGlyph`, whose only writer is the canvas, which a panel window never renders. | New `DetachedGlyphSource` runs the canvas's own data path — `useGlyphDetail`, `useFontInfo`, `buildScene`, `useLayerOutline`, with the same outline merge — and publishes the glyph; `DetachedPanel` mounts it for a glyph tab. `DetachedPanel.test.tsx` covers the Inspector, the sidebar following a glyph change, and publishing nothing for the Overview. A Playwright case detaches the Inspector **from an Edit tab**, reads its LSB, steps the owning window to another glyph and watches the value follow.                                                                                                                                                      |
| 3   | **Row 1.** `aria-keyshortcuts` was set on every document's tabs while `Mod+Alt+<n>` resolves against the active document.                                                                                           | `ordinalShortcut` takes the active document and returns nothing for another document's tabs. `TabBar.test.tsx`: only the active document's tabs carry the attribute.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 4   | **Row 1 a11y.** The pinned state was not exposed at all (icon `aria-hidden`, `data-pinned` a test hook, `title` ignored on a button with text).                                                                     | The tab carries `aria-description` with the translated "Pinned", keeping the accessible _name_ the tab title. `TabBar.test.tsx` asserts the description and that the name is unchanged.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 5   | **Row 8.** `scope.rs` returned English `Master/Background/…` that the interface printed verbatim.                                                                                                                   | `LayerScope` is now `Named(name)` or `Role(LayerRole)`; the API carries `HistoryLayer { name, role }`; `HistoryPanel` translates the role through `history.layerRole.*` and falls back to a generic label for a role it does not know. Rust and Vitest cases for both.                                                                                                                                                                                                                                                                                                                                                                                                  |
| 6   | **Row 8.** A per-glyph COLR/SVG/MATH edit reported only `Font-level`.                                                                                                                                               | `changed_table_glyphs` diffs the per-glyph records of the color, SVG and math snapshots, so the glyph whose record moved is named alongside the font-level scope. Three Rust cases, including one where an unchanged table names no glyph.                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 7   | **Row 7.** The determinism test built its changed set from the post-edit snapshot only, so it could not see the delete.                                                                                             | It now asserts the added, removed and rewritten sets separately: one glyph revision added, one pruned, `project.json` rewritten, file count unchanged. The wording below is corrected too.                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 8   | `doc.history_go` could leave the cursor half-moved when a step was refused.                                                                                                                                         | `replay_to` walks its own steps back on failure and reports the original error; if the way back is refused too, the error names where the cursor stopped. `crates/fc-api/tests/history.rs` covers a refused replay (read-only document) leaving the cursor and the font untouched, plus the stale and out-of-range rejections.                                                                                                                                                                                                                                                                                                                                          |
| 9   | `view.previewCollapse`'s `when` omitted `tab.textState` while `run` returned early on it — a silent no-op.                                                                                                          | `when` requires the text state. Covered in `workspaceTabCommands.test.ts`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 10  | `guarded()` wrapped the whole application life, so any later panic was swallowed and mislabelled "could not start".                                                                                                 | The builder is split: only `Builder::build()` runs inside the guard, the default hook is restored before `App::run`, and a panic once the application is running behaves normally. The five binary cases still pass.                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 11  | `WorkspaceDialog` read `workspaceStorageOf` without subscribing, so the storage sentence never updated.                                                                                                             | The storage kind lives in `workspaceStore` (`storage`, `setStorage`) and the dialog selects it. `WorkspaceDialog.test.tsx` asserts the text changes when the kernel answers.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| +   | **From task d.** Reopening a saved file showed two pinned Overview tabs for one file.                                                                                                                               | Investigated, and **not** a duplicate: Overview insertion is idempotent per document, and two tabs appear because two _documents_ of one file are open — which the P2a handover case deliberately creates and `openPathThroughDialog` deliberately waits for. Making `open` reuse an already-open path was tried and reverted: it broke two accepted P2a cases that open a file they have just saved. Pinned instead by three Vitest cases (a second open is a second document with one Overview each; reopening the same _document_ adds nothing; a closed file reopens as a new document) and a Playwright case that opens one file twice and counts the pinned tabs. |

`cargo clippy -p fc-api --all-targets` also reports `needless_update` in
`crates/fc-api/tests/rescale_upm.rs:69`, which belongs to the export area; the
library and this task's own tests are clean.

### The P2a suite after these fixes

`FC_API_PORT=8801 FC_UI_PORT=1501 pnpm --dir app exec playwright test --config
e2e/p2a.config.ts`: **10 of 11 pass**, including the case the earlier report
named, `authors groups and pair exceptions…`. One case still fails,
`saves a pending edit and keeps document handover isolated`, and not because of
anything this task changed: `app/e2e/p2a-kerning.spec.ts:322` clicks
`getByRole('tab', { name: 'Regular.ufo', exact: true })`, while a pinned Overview
with no label has been rendered as `Overview · <file>` since the 2026-09-30 batch
(the same line at `HEAD` reads identically). That spec belongs to the metrics
area; matching `(?:^|· )Regular\.ufo$`, as their own `openPathThroughDialog`
already does, is the one-line fix. Verified by reading the failure's page
snapshot, which lists the tab as `"Overview · Regular.ufo"`.

## Limits

- **macOS and Windows are not executed.** Row 9 stays Partial for that reason
  alone, and rows 1 and 2 claim Linux plus a compiled, unit-tested native path.
- **Native desktop windows were not run.** `src-tauri`'s window routing is
  covered by its own tests and `cargo check`; opening a real Tauri window needs a
  desktop session, which this task did not use.
- **Two-worker Playwright timing.** `app.spec.ts` and `panels.spec.ts` drive one
  shared kernel document. Serial (`--workers=1`) they pass; with two workers the
  grid can miss its 5 s budget while the other worker edits. The integrator may
  prefer `fullyParallel: false` or `workers: 1` in `playwright.config.ts`, which
  is not this task's file.
- **Workspaces of a non-native document stay local.** An imported `.ttf` has no
  FontCarver storage to put them in. The Workspaces dialog says which of the two
  applies; saving the document as `.fcf` or `.fcfp` moves them into the project.
- **Reopening detached windows can be refused.** Restoring an arrangement at
  document-open time is not a user gesture, so a browser may block the popups;
  the shell then says so once instead of once per window. The Tauri path has no
  such restriction.
- **The history list is capped at 500 entries** and says when older entries were
  dropped. The cap is a response-size bound, not a limit on undo itself.
- **Recovery snapshots are periodic**, at the `autosave_seconds` preference
  (60 s by default). Work done in the last interval before a crash is not in the
  snapshot. This is the pre-existing design, restated here because row 7 asks
  about recovery.
- **The identifier change is not propagated into the release documentation.**
  `docs/releasing.md` still names `com.fontcarver.app` in two places (lines 219
  and 233, the second being the warning that is now gone). That file belongs to
  the release area. `scripts/check_release_bundle.py` reads the identifier out of
  the built bundle rather than hard-coding it, so it needs no change. An installed
  copy of a previous build keeps the old identifier; on macOS and Windows that
  means the new bundle installs beside it rather than over it, which is the
  expected consequence of changing an application identifier once.
- **A layer with no name is labelled by its role** — `Master`, `Background`,
  `Intermediate`, `Alternate`, `Smart`, `Backup` — because the alternative is a
  UUID. A multi-master font therefore says `Layers: Master` rather than naming
  the master; resolving master names would need the history API to carry them,
  which it deliberately does not.

## Resource use and cleanup

Artifact budget at the start: build 6.00 GiB / 12,708 files, temporary 0.00 GiB,
free 666.53 GiB; **PASS**. Mid-task: build 9.93 GiB / 16,768 files, then 18.87
GiB / 21,977 files at its peak, with the 10 GiB warning and a shared
`cargo clean` by the orchestrator in between; **PASS** throughout. After
the review fixes and cleanup: build 7.67 GiB / 15,927 files, temporary 0.01 GiB /
1,467 files, free 631.18 GiB; **PASS**, the 10 GiB warning gone after the
orchestrator's shared cache maintenance. The remaining temporary files
belong to the other P5f areas still running.

This task cleaned only its own outputs — its scratch directory (including the
`.fcfp` and `.fcf` fixtures it created, its dev server's configuration and
recovery directories, and its `TMPDIR`), the Playwright attachments under
`app/test-results`, and its report directory — and stopped every process it
started: the dev server on 8801, Vite on 1501 and its headless browser session.
It did not touch `target/`, which seven other areas were building into
throughout, and did not run `cargo clean`: the orchestrator maintains the shared
cache.
