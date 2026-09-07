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
