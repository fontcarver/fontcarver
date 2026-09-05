# P1 Task 9a report — dialogs, progress line, recovery banner, `glyphs.add` dry run

Branch `p1-t9a` in `/home/chen/githubprojects/pixelfonts/fontworkshop/.worktrees/p1-t9a`, eight commits on
top of `e107896`. Not pushed. Ports used: kernel 8792, Vite 1425.

```
b29ffd5 feat(fw-api): add an additive dry_run flag to glyphs.add
3816cfe feat(app): dialog primitive, preference and progress stores
83d7dc7 feat(app): Add Glyphs dialog with a kernel dry-run preview
57eec91 feat(app): Export dialog with format tabs, options and a warning report
0a63642 feat(app): Preferences and Recovery dialogs
d8821db feat(app): wire the dialogs into the menu, shortcuts and shell
4c32e4c test(app): end-to-end specs for the export exit criterion and the dialogs
61adb57 feat(app): top-align stacked dialog rows and explain the metrics presets
```

## Kernel change (ruling 2)

`AddGlyphsParams.dry_run: bool` with `#[serde(default)]`
(`crates/fw-api/src/types/editing.rs:100`); the branch is
`crates/fw-api/src/handlers/editing.rs:107`. A dry run resolves the recipe with the same grammar,
the same duplicate check and the same expansion limits as a real call, then returns the previewed
cells without applying a command or touching the undo stack. `plan_additions` now carries each
planned name with its Unicode value so `preview_cell`
(`crates/fw-api/src/handlers/editing.rs:177`) can build a cell for a glyph that does not exist yet;
its handle is the documented sentinel `PREVIEW_GLYPH_ID = GlyphId(u32::MAX)`
(`crates/fw-api/src/types/editing.rs:79`), deliberately outside any session's range so a caller that
forwards a preview cell to a glyph-scoped method gets a clean "does not exist" error. Callers key a
preview by name. `glyph_classification` became `pub(crate)`. Two new integration tests
(`crates/fw-api/tests/editing.rs:328` and `:364`) assert that a dry run previews the right names,
leaves `glyph_count`, `dirty` and `undo_label` untouched, and reports the same names and skips as the
real call. `cargo fmt/clippy/test -p fw-api` green; `pnpm --dir app gen:api` regenerated.

## Shared infrastructure

| What | Where |
|---|---|
| `Dialog` primitive: 6 px radius, 24 px padding, 16/600 title, focus trap, Escape cancels, Enter confirms, focus restored on close, three spec widths (480/560/640) as classes rather than inline styles | `app/src/components/Dialog.tsx:40`, classes appended to `app/src/shell/dialog.module.css` |
| Dialog slot in the UI store: one typed `DialogState` union, so a second dialog replaces the first instead of stacking focus traps | `app/src/stores/uiStore.ts:54` (`DialogState`), `setDialog`, `dismissRecoveryBanner` |
| `DialogHost` mounts the open dialog | `app/src/dialogs/DialogHost.tsx:15` |
| `progressStore`: subscribes to `Event.kind === "Progress"`, clears on the terminal `write 100` / `failed 100`, pushes the failure message as an error toast, and `begin()` shows a line before the first kernel event arrives | `app/src/stores/progressStore.ts` |
| `ProgressLine`: the 2 px `--color-accent` line under the tab bar, positioned out of flow so an idle shell reserves no gutter | `app/src/shell/ProgressLine.tsx`, `.progressLine`/`.progressFill` in `AppShell.module.css` |
| `prefsStore`: `prefs.get` / `prefs.set`, applies optimistically and rolls back on a rejected write, mirrors theme and language into the shell | `app/src/stores/prefsStore.ts` |
| `RecoveredBanner`: dismissible, driven by `DocSummary.recovered_from` | `app/src/shell/RecoveredBanner.tsx` |
| Shell wiring: progress subscription, `prefs.get`, `checkForRecovery()` at start-up; `ProgressLine`, `RecoveredBanner` and `DialogHost` mounted | `app/src/shell/AppShell.tsx:58`, `:63`, `:89`, `:104`, `:128` |

`ToastStack`/`Toast` were not touched; the dialogs only call `pushToast`.

## Per dialog

### New Font — `Mod+N`, 480 px
`app/src/dialogs/NewFontDialog.tsx:30`, mapping in `app/src/dialogs/newFont.ts`.
Family, style, units per em (1000 / 2048 / custom), metrics preset (Latin default; choosing CJK pins
the em to 1000), starter set (none / Basic Latin / Latin-1 / GB 2312).
API: `session.new { family, units_per_em }` → `doc.font_info` (the batch replaces the whole
`FontInfo`, so the current record is read first) → one
`doc.apply { Batch: [SetFontInfo, SetMasterMetrics] }` → `glyphs.add` with the starter recipe
(`newFont.ts:21`; Basic Latin is `A-Z a-z 0-9` plus the ASCII punctuation ranges, Latin-1 is
`U+0020-U+00FF`, GB 2312 is `@charset:GB2312`) → `refreshSummary`. The document opens in a Font tab
through the existing `sessionStore.createDocument`.
Latin metrics scale with the em (0.8 / −0.2 / 0.5 / 0.7); CJK is 880 / −120 at 1000 upem. Every
per-master override stays `null` so the compiler derives hhea/OS/2/vhea.

### Add Glyphs — `Mod+Shift+G`, 560 px
`app/src/dialogs/AddGlyphsDialog.tsx:43`, helpers in `app/src/dialogs/addGlyphs.ts`, virtualised list
in `app/src/dialogs/NameList.tsx`.
Recipe textarea with syntax help, a character-set picker with live counts and "already present"
badges, and a virtualised preview of the names that would be created (only the visible window is
mounted, so a GB 2312 preview of ~6,800 rows stays cheap). Every number comes from
`glyphs.add … dry_run: true`, debounced 200 ms; "Add N glyphs" repeats the same recipe with
`dry_run: false`, refreshes the summary and toasts the count. The picker lists Basic Latin, Latin
Extended-A, Latin-1 and GB 2312 and fetches their counts sequentially (cheapest first); other
repertoires are reachable by typing `@charset:Big5`, `@charset:JISX0208` or `@charset:KSX1001`, which
the syntax help mentions.

### Export — `Mod+E`, 560 px
`app/src/dialogs/ExportDialog.tsx:77`, helpers in `app/src/dialogs/exportFormats.ts`.
Format tabs TTF / WOFF / WOFF2 / UFO / FWF, with Designspace added only when the document has more
than one master (`exportFormats.ts:38`). Switching the tab follows into the suggested file extension
unless the user typed their own path. Destination goes through the same platform abstraction P0 uses
for Save (`chooseExportPath`, `exportFormats.ts:93`): under Tauri the native save dialog runs behind
a "Choose…" button; under the HTTP transport there is no native dialog, so the button is not rendered
and the field carries the same "type a path the kernel can write" hint the P0 path prompt uses.
Options are the four the design lists, each with a one-line explanation, mapped to
`ExportOptions.remove_overlap`, `keep_tt_instructions`, `preserve_untouched_tables` and
`production_names`; they are shown only for the sfnt formats, since UFO/Designspace/FWF go through
the source and document writers. `ExportOptions.autohint` and `table_subset` exist but are not
exposed (autohint is P5 and the kernel warns that it was not applied; a table allowlist is not a P1
dialog concern) — they are sent as `false` / `null`.
"Export" runs `session.export`, disables the button, and calls `progressStore.begin()` so the busy
line appears before the kernel's first `compile` event. The result view lists the path, bytes, edited
glyphs and timing, then the `ExportReport` warnings translated through the existing warning i18n
(`app/src/i18n/warnings.ts`, extended with 15 export codes). Affected glyph names render as chips;
clicking one runs `glyphs.search` and selects that glyph in the document's Font tab, or does nothing
when the tab is closed (`ExportDialog.tsx:53`).

### Preferences — 640 px
`app/src/dialogs/PreferencesDialog.tsx:34`. Left rail General / Editor / Snapping. General: theme,
language (switches i18next live), autosave interval. Editor: the three nudge steps and the grid size.
Snapping: the five `SnapPrefs` toggles. Every change is applied and persisted immediately through
`prefs.set`; "Reset to defaults" writes `DEFAULT_PREFS`. The View-menu theme commands now go through
the same store (`builtinCommands.ts:56`) so the menu and the dialog write one record.

### Recovery
`app/src/dialogs/RecoveryDialog.tsx:21`, launch check in `app/src/dialogs/recovery.ts:12`.
`session.recover_list` runs once at start-up and opens the dialog only when it is non-empty; a failure
there is deliberately silent. Each row shows the original path (or "Untitled document") and the
snapshot time. "Recover" calls `session.recover` through two new session-store actions
(`sessionStore.recover` / `sessionStore.discardRecovery`), opens the document in a Font tab and — via
`DocSummary.recovered_from` — shows the dismissible banner "Recovered from … — saving overwrites the
original." "Discard" asks once in the row before `session.discard_recovery`. The dialog closes when
the last row is dealt with. The whole flow works over the HTTP dev transport.

## Menu and shortcut wiring

Additive hunks in `app/src/shell/builtinCommands.ts`: `file.new` now opens the dialog (`:73`),
`file.export` `Mod+E` (`:144`), `glyph.add` `Mod+Shift+G` (`:209`), `edit.preferences` `Mod+,`
(`:223`). `MenuBar.tsx` was not modified — the commands appear from the registry. `EmptyState`'s
"New Font…" now runs the same command instead of creating an untitled document directly.

## Verification

```
cargo fmt --all --check                                   OK
cargo clippy -p fw-api --all-targets -- -D warnings        OK
cargo test -p fw-api                                       103 passed (64+2+11+10+8+2+6), 0 failed
pnpm --dir app gen:api                                     OK
pnpm --dir app check                                       OK
pnpm --dir app test                                        48 files, 357 tests passed
pnpm --dir app lint                                        OK
pnpm --dir app exec prettier --check .                     OK
pnpm --dir app e2e                                         14 passed (9 pre-existing + 5 new)
```

The e2e run needs the kernel started by hand until Task 11 makes the config self-contained:

```
FW_CONFIG_DIR=<tmp> ./target/debug/fw-devserver --port 8792 --open fixtures/fonts/DejaVuSans.ttf
FW_DEV_PROXY=http://localhost:8792 VITE_FW_API=http://localhost:1425 FW_UI_PORT=1425 pnpm --dir app e2e
```

New specs. `app/e2e/export.spec.ts` — the P1 exit criterion: opens `DejaVuSansMin.woff2`, applies one
`MovePoints` on `A` through the API from the page and re-reads the outline to prove the point moved,
opens the Export dialog from the File menu, picks TTF, exports to a temp path, then asserts the file
exists and that `fw inspect --json` reports the same glyph count (53). `app/e2e/dialogs.spec.ts` —
New Font with Basic Latin (status bar ≥ 52 glyphs), an Add Glyphs dry-run preview whose count (11) is
exactly what the applied call adds, a preferences round trip through the kernel including a live
language switch, and an empty recovery listing with no dialog on a clean launch. `app/e2e/support.ts`
holds the shared `api()`, `fixture()` and `inspect()` helpers. Preferences are kernel-wide, so that
spec restores the original record in a `finally`. `playwright.config.ts` was not touched.

New Vitest files: `dialogs/{newFont,exportFormats}.test.ts`,
`dialogs/{NewFontDialog,AddGlyphsDialog,ExportDialog,PreferencesDialog,RecoveryDialog,DialogHost}.test.tsx`,
`stores/{prefsStore,progressStore}.test.ts` — 51 tests covering each dialog's parameter mapping
through the mock transport, the shortcut/menu wiring, the busy line and the banner. `App.test.tsx`
gained `prefs.get` / `prefs.set` / `session.recover_list` handlers now that the shell calls them.

## Interactive pass

Kernel on 8792 with `--open fixtures/fonts/DejaVuSans.ttf`, Vite on 1425, agent-browser at 1440×900.
Both servers stopped afterwards. Screenshots in `docs/screenshots/`:

- `p1-dialog-newfont.png` — New Font at 480 px over the DejaVu Sans grid: family, style, the three
  units-per-em choices, the Latin/CJK preset with its explanation, and the four starter sets.
- `p1-dialog-addglyphs.png` — Add Glyphs with the recipe `uni4E00-uni4E2F a.sc f_i`: the character-set
  picker showing live counts (Basic Latin "0 new / 95 already present", GB 2312 "7,140 new / 305
  already present") and the virtualised preview listing 50 new names with their code points.
- `p1-dialog-export.png` — an export of DejaVu Sans in flight: the accent progress line under the tab
  bar at the `preserve` stage (60 %), the Export button disabled and reading "Exporting…".
- `p1-dialog-export-report.png` — the finished report: path, 752,716 bytes, 0 edited glyphs,
  27,633 ms and the translated `mac-name-records-dropped` warning.
- `p1-dialog-prefs.png` / `p1-dialog-prefs-editor.png` — Preferences at 640 px, General and Editor.
- `p1-dialog-prefs-zh.png` — the same dialog after switching the language to 简体中文: the menu bar,
  sidebar, inspector and status bar switch live; switching back to English was verified in the same
  session. The dialog's own labels stay English because the new zh-CN keys are placeholders (below).
- `p1-dialog-recovery.png` / `p1-recovered-banner.png` — the recovery dialog after a real autosave
  snapshot (kernel restarted with `autosave_seconds: 5`), and the recovered document's banner.

## New i18n keys (for Task 13)

135 new keys in `app/src/i18n/en.json`; the same keys exist in `zh-CN.json` with **English
placeholders** and were not translated. Groups: `dialog.newFont.*` (incl. `preset.*`,
`presetHint.*`, `set.*`), `dialog.addGlyphs.*` (incl. `charset.*`), `dialog.export.*` (incl.
`formatName.*`, `option.*`), `dialog.prefs.*` (incl. `category.*`, `nudgeStep.*`, `snap.*`),
`dialog.recovery.*`, `cmd.file.export`, `cmd.glyph.add`, `cmd.edit.preferences`, `common.{add, done,
discard}`, `progress.{compile, preserve, write}`, `a11y.progress`, `banner.{recovered, dismiss}`,
`undo.newFont`, `toast.glyphsAdded_*`, and 15 `warning.*` keys for the export diagnostics.

## Deviations and concerns

1. **"Reveal in folder" omitted.** Nothing in the app can open a file manager: `@tauri-apps/plugin-opener`
   is not a dependency and adding it would mean a new npm and Rust dependency plus a capability
   change, outside this task. Ruling 3 allows omitting it with a note — this is that note.
2. **Warning chips are parsed out of the message.** `ExportWarning` carries only `code` and `message`;
   the affected glyphs are formatted into the text by `fw-compile`. `warningGlyphNames`
   (`exportFormats.ts:75`) therefore parses the tail of `hinting-dropped` messages only, and returns
   nothing unless every token looks like a glyph name. A structured `glyphs: Vec<GlyphName>` on
   `ExportWarning` would be the honest fix; it was left out to keep the Rust change to the one
   additive flag the dispatch allowed.
3. **`Mod+,` for Preferences** is not in the §11 keyboard map, which lists no shortcut for the
   dialog. The platform-standard binding was chosen; say the word if it should be dropped or moved.
4. **Preview cells carry a sentinel handle.** A dry-run `GlyphCell.gid` is `u32::MAX`. Documented on
   both sides, and the UI keys the preview by name, but any future consumer must not treat it as a
   handle.
5. **Theme changes now cross the kernel.** Both the View menu and Preferences write through
   `prefsStore`, so a failed `prefs.set` rolls the theme back and toasts. That is the intended "never
   claim a preference the kernel did not store" behaviour, but it does make a menu toggle depend on a
   kernel round trip.
6. **`Prefs` has no hit/snap distance field**, so the "hit/snap distances" the design lists for the
   Editor category are limited to the grid size and the snapping toggles. Adding a field would have
   been a second, non-additive kernel change.
7. **Preferences and open documents are global to a kernel session**, which makes e2e specs
   order-sensitive; the preferences spec restores the record in a `finally` and every other new spec
   works on a document it creates itself. Task 11's self-contained Playwright setup should keep that
   in mind.
8. **`dialog.module.css` was extended** (three width classes and a scrolling body) and
   `AppShell.module.css` gained the progress line and banner rules; `AppShell.tsx`,
   `builtinCommands.ts`, `EmptyState.tsx`, `sessionStore.ts`, `uiStore.ts`, `warnings.ts` and
   `App.test.tsx` have small additive hunks. `MenuBar.tsx`, `TabBar.tsx`, `ToastStack`, `Toast`,
   `playwright.config.ts`, `app/src/views/**` and `app/src/editor/**` were not touched.

---

# Fix round 1

Review: `task-9a-review.md` (Needs fixes — 3 Important, 11 Minor). Six commits on top of `61adb57`;
no Rust was touched in this round (`git diff 61adb57..HEAD -- crates/ src-tauri/` is empty).

```
082b0c2 fix(app): give the Dialog primitive its spec chrome and modal keyboard ownership
36eb909 fix(app): close a half-made new font and never write defaults over stored prefs
7687cba fix(app): explain every skipped recipe item in Add Glyphs
8533e8a refactor(app): one native-file-dialog helper for open, save and export
c6a6195 fix(app): commit preference numbers once, and confirm every dialog with Enter
232a0c7 docs: refresh the P1 dialog screenshots for the new dialog chrome
```

## Important

**I1 — a failed New Font left a half-initialised document.** `createFont` now runs everything after
`session.new` inside `setUpDocument` (`app/src/dialogs/newFont.ts:96`) and, on any rejection, closes
the document through `sessionStore.close` before rethrowing (`newFont.ts:87-91`). The dialog stays
open with its error toast, no tab survives, and a retry mints exactly one more document.
*Test:* `NewFontDialog.test.tsx:161` — a rejecting `doc.apply` produces a `session.close` for that
document, an empty `docs`/`tabs`, an error toast, an open dialog, and a second Create that closes its
own document rather than accumulating orphans.

**I2 — the Dialog primitive did not have the §9 chrome it claimed.** 24 px padding, a 16/600 title
and a 14 px body are now real, written as three rules scoped to the primitive's own width classes
(`app/src/shell/dialog.module.css:126-152`), so the P0 path prompt, close prompt and command palette
keep the denser chrome they were designed with. Two additive tokens carry the sizes
(`app/src/styles/tokens.css:24-26`: `--text-size-md: 14px`, `--text-size-xl: 16px`). The doc comment
on `Dialog` now says what the code does (`app/src/components/Dialog.tsx:31-38`).
*Test:* `Dialog.test.tsx:46-68` resolves each declaration against the token file and asserts 24 px /
16 px / 600 / 14 px for all three widths, plus that the unscoped P0 rules still read 12/16 px and
15 px.

**I3 — a preference write before a successful load persisted the defaults.** `update` now reads the
kernel's record first when the store has not loaded, and refuses the write outright when that read
fails (`app/src/stores/prefsStore.ts:102-108`); `reset` is the one write that deliberately means "the
defaults", loaded or not.
*Tests:* `prefsStore.test.ts:91` (a write before load issues `prefs.get` first and keeps the stored
`recent_limit`/`autosave_seconds`), `:111` (an unreadable record produces no `prefs.set` and an error
toast), `:126` (reset still writes the defaults).

## Minor

| # | Change | Where | Test |
|---|---|---|---|
| 1 | Warning chips capped at 50 with a "+N more" summary | `ExportDialog.tsx:47`, `:329-341` | `ExportDialog.test.tsx:190` — 120 names render 50 chips and "70 more" |
| 2 | Dead i18n keys deleted (`dialog.addGlyphs.present`, `common.add`, `common.discard`), plus `presentCount_*` and `skipped_*` which the new grouping replaced | `i18n/{en,zh-CN}.json` | `i18n.test.ts` parity; no `t()` reference remains (grepped) |
| 3 | `skipped` is grouped by reason — already present / not understood / range too large — using the kernel's own discriminator (`too-large:` prefix, then its glyph-name grammar) | `addGlyphs.ts:28-70`, `AddGlyphsDialog.tsx:210-225` | `addGlyphs.test.ts` (8 classifications + grouping), `AddGlyphsDialog.test.tsx:92` |
| 4 | Charset rows label their counts by reason instead of calling every skip "already present" | `AddGlyphsDialog.tsx:181-196` | `AddGlyphsDialog.test.tsx:166` |
| 5 | `onConfirm` wired for Add Glyphs (add), Preferences (Done) and Recovery (recover the focused row, else Later) | `AddGlyphsDialog.tsx:139`, `PreferencesDialog.tsx:75`, `RecoveryDialog.tsx:52` | `AddGlyphsDialog.test.tsx:122`, `PreferencesDialog.test.tsx:141`, `RecoveryDialog.test.tsx:104` and `:119` |
| 6 | One `choosePath({ kind: 'open' \| 'save', … })` helper; the only place the app reaches for `@tauri-apps/plugin-dialog` | `app/src/shell/nativeDialogs.ts`, used by `sessionStore.ts:175-217` and `exportFormats.ts:90-104` | `nativeDialogs.test.ts` (4 cases, incl. the mocked native picker), existing `builtinCommands`/`sessionStore` tests for the in-app fallback |
| 7 | A charset whose dry run failed shows "—" with a tooltip instead of counting for ever | `AddGlyphsDialog.tsx:94-99`, `:183` | `AddGlyphsDialog.test.tsx:105` |
| 8 | Preference numbers keep a local draft and commit on blur or Enter, so an emptied box never persists `0` | `app/src/dialogs/NumberField.tsx` | `PreferencesDialog.test.tsx:105` (one write, on blur) and `:127` (Enter commits without closing) |
| 9 | `dismissedRecoveries` is pruned when a document closes | `uiStore.ts:315-322` | covered by `uiStore.test.ts` close-tabs cases; the list can no longer grow for the session |
| 10 | Global shortcuts stand down while a modal is on screen; Escape is exempt. The primitive counts itself in and out through `uiStore.modalCount` | `shortcuts.ts:186-197`, `Dialog.tsx:56-60`, `uiStore.ts:170,355-359` | `shortcuts.test.ts:178` (`Mod+W` is ignored under a modal and works again after), `:193` (Escape still dispatches), `Dialog.test.tsx:107` (the count is balanced) |
| 11 | `warningGlyphNames` parsed once per report rather than twice per warning per render | `ExportDialog.tsx:151-159` | covered by the chip tests above |

Two supporting changes fell out of the above: the Add Glyphs preview list and the recovery rows are
now focusable (`NameList.tsx:52`, `RecoveryDialog.tsx:81`), which is both what makes Enter reachable
and the right thing for a scrollable region and a row that carries two actions. `test-results` was
added to `app/.prettierignore` so a Playwright artefact left by a failed run cannot fail
`prettier --check .`.

## Verification

```
cargo fmt/clippy/test -p fw-api        not re-run: no Rust changed this round (diff is empty)
pnpm --dir app check                   OK
pnpm --dir app test                    51 files, 387 tests passed (was 48/357: +3 files, +30 tests)
pnpm --dir app lint                    OK
pnpm --dir app exec prettier --check . OK
pnpm --dir app e2e                     14 passed (9 pre-existing + 5 new), servers on 8792/1425
```

The e2e Preferences spec needed no change: it already blurred the grid-size field before polling, so
commit-on-blur is exactly what it asserts.

## i18n changes for Task 13

Removed (do not translate): `common.add`, `common.discard`, `dialog.addGlyphs.present`,
`dialog.addGlyphs.presentCount_one/_other`, `dialog.addGlyphs.skipped_one/_other`.
Added (English placeholders in `zh-CN.json`): `dialog.addGlyphs.countFailed`,
`dialog.addGlyphs.skip.{present,unknown,tooLarge}_one/_other`,
`dialog.export.moreGlyphs_one/_other`. Net 547 leaf keys per catalogue, parity exact.

## Screenshots

All eight were re-shot against a single-document session with the new chrome, and
`p1-dialog-export-form.png` was added so the export dialog has both a resting frame and the
mid-export one (`p1-dialog-export.png`, caught at `preserve 60`). `p1-dialog-addglyphs.png` now shows
the grouped skip counts ("50 new · 1 not understood") and the per-reason charset badges.

## Concerns

1. **Enter in Add Glyphs is reachable only from the preview list.** The recipe field is a textarea and
   everything else is a button, and both own Enter themselves; making the list focusable is what gives
   `onConfirm` a real path, and that is what the test drives.
2. **Export still falls back to its own destination field, not `PathPromptDialog`.** The helper is
   shared, but stacking the P0 prompt over an open Export dialog would put two modals and two focus
   traps on screen at once, and Export already has a persistent path field. Flag it if the review
   wants the prompt reused there anyway.
3. **The skip-reason discriminator mirrors the kernel rather than being told by it.** It is exact
   today (every planned name is a valid glyph name; only oversized ranges are prefixed), but it is a
   parallel copy of `is_glyph_name`. A `reason` field on the skip entries would make it structural —
   deliberately not done, since the dispatch allowed exactly one additive Rust change.
4. **`Prefs` still has no hit-radius or tool-defaults field**, so those parts of §9's Editor category
   remain unimplementable; the review noted tool defaults, and it is recorded here for Task 13.
