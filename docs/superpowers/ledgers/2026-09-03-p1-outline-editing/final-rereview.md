# Final scoped re-review — 2026-09-07

Original whole-branch candidate: `62348254bbdf8a141bb771919881303238b7916e`; base: `cd12fd6c108221931318deaab1bbdb5ad11dccca`. Only F1/F2/F3 corrections and directly affected validation are in scope. No overall approval yet.

## F1 interim candidate 491cfce281ff412b6c0c53266e3c921b7ecbb995

**Not approved.** The new `withSettledDocument` checks `editor.frame().source?.doc`, but the actual `EditorFrame` has no source field. It never prepares a real editor; the new test double invented this field and consequently conceals the failure. Independent `pnpm --dir app check` fails with TS2339 at `documentPersistence.ts(15,21)`: Property 'source' does not exist on type 'EditorFrame'. Root notified immediately. Correct the source identity access and add an actual-controller regression proving an unfinished Pen contour is committed before persistence. The barrier sequencing approach itself is appropriate; save/export/dirty-close/discard routing is within F1 scope.

F2 and F3 correction candidates pending.

## Integrated corrections 2e53ba6432b9bee3aa15000a7918f472c9289fa9

F1 corrected by `8e6f383`: explicit real `EditorController.documentId` replaces the nonexistent frame field. Actual-controller regressions now cover Pen completion before saving and metrics ownership, with document barriers captured before persistence is published. Save/export/close semantics inspected; explicit discard does not save and dirty decisions use a fresh kernel summary after closure.

F2 `37e39f7`: `face_with_order` accepts exactly one matching source face, declining raw-outline/instruction reuse on ambiguity. Selected model geometry and selected raw layout remain available; existing hinting-drop warning applies. Independent `cargo test -p fc-compile --test preserve` at the integrated candidate: **9 passed, 0 failed/ignored**, including the generated ambiguous-TTC regression; 109.30 s under concurrent build load.

F3 `1ff0101`: GlyphView captures originating tab/document/glyph, prepares old editor tools before clearing edited layer/changing master, and revalidates after await. The inner toolbar now invokes that guarded callback. Actual TextView test holds AddContour pending, verifies the old master/layer remains installed, then observes exactly one contour applied to the old layer before switching.

Independent `pnpm --dir app check` now passes. First seven-file scoped run: 54 passed / 3 failed, all failures in pre-existing async GlyphView/TextView checks during simultaneous Rust/full-app builds (49.31 s, observed system load 43.49 on 20 CPUs). New F1/F3 regressions passed. Unchanged rerun deferred until builds drain; this is not yet final approval. Performance test correction inspected: successful captured `doc.edit_end` is awaited outside the timed pointer-to-paint measurement; the 8 ms threshold, exact coordinates and undo assertions remain intact.

Visual evidence additionally inspected: headless `p1-text-edit-in-place.png` and `p1-text-stale.png` from `5db96c7`, showing active third-glyph editing chrome, model preview at three sizes, light/dark strip and recompilation state. These screenshots do not establish native desktop interaction.

## Final scoped verdict — approved

**F1, F2 and F3 are resolved at production candidate `2e53ba6432b9bee3aa15000a7918f472c9289fa9`. No remaining confirmed blocker from the whole-P1 review.** Later root documentation/snapshot/script commits contain no changes to reviewed application/Rust/Tauri production sources.

After all heavy builds drained, the exact same seven-file command was rerun with unchanged tests, timeouts and two-worker cap: **7 files / 57 tests passed in 4.04 s** (versus 49.31 s during contention). All three earlier async failures pass, as do the new actual-controller persistence and delayed old-master Pen regressions. Command: `pnpm --dir app exec vitest run src/editor/documentPersistence.test.ts src/stores/sessionStore.test.ts src/shell/tabActions.test.ts src/dialogs/ExportDialog.test.tsx src/views/text/TextView.test.tsx src/views/text/TextView.master.test.tsx src/views/glyph/GlyphView.test.tsx --maxWorkers=2`. Independent TypeScript check and nine Rust preservation tests also passed, as recorded above. No implementation changes were made by the reviewer.

This completes the one fix wave and scoped re-review. Root's final full app suite and serialized fresh-release headless acceptance run still determine final acceptance completion; approval here does not replace them. Root/Sol report final release and embedded Tauri builds passed without native launch. Existing native workflow remains paused/unverified by user instruction. Preserve accepted P1 limitations, including ambiguous-TTC hinting fallback, narrow Source Han export headroom and model-outline preview. Finish P1 then stop; main merge remains separate.
