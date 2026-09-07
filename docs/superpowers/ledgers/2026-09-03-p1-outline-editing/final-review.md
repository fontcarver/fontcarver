# Whole-P1 independent review — 2026-09-07

**Verdict: needs fixes.** Reviewed candidate `62348254bbdf8a141bb771919881303238b7916e` against merge-base `cd12fd6c108221931318deaab1bbdb5ad11dccca`. Three confirmed blocking integration defects. Implementation remained read-only; reproductions and this report are in the ignored ledger. This is whole-branch review, not an aggregation of task approvals.

## Confirmed blockers

1. **F1 / High — persistence bypasses edit ownership.** `app/src/stores/sessionStore.ts:209` calls `session.save` directly; `:255` and `:268` close/decide dirty without settling. `Document::save` finalizes the active token. Reproduction using actual store/TextMetricsRunner: apply metrics key (width 601), Save, another key within 500 ms produces `InvalidEditToken`; width remains 601 instead of 602. A second probe proves `session.close` overtakes an unresolved document barrier. `ExportDialog.tsx:153` similarly snapshots without finishing local tools. Settle the matching editor and document barriers before persistence/dirty decisions; preserve explicit Discard semantics. Probe: `final-review-probe.test.ts` / `.config.mjs`, two bug-demonstration tests passed.

2. **F2 / High — wrong TTC face exported silently.** `crates/fc-compile/src/preserve.rs:293` selects the first original face matching glyph order. Two faces commonly share order. A licensed subset-derived collection with A shifted 70 units in face 1, imported with `font_index: Some(1)`, exports face 0's A `(700,1294)` instead of selected face 1 `(770,1294)` under defaults. No preservation warning. Require unambiguous original-face identity or conservatively decline raw outline/instruction preservation. Production-rlib probe: `final-review-ttc.rs`, `review-twoface.ttc`, `review-face1-export.ttf`.

3. **F3 / High — inner master selector loses an unfinished path.** `app/src/views/glyph/GlyphToolbar.tsx:83` changes master directly. In integrated TextView, child source clearing precedes the parent's asynchronous master bridge; Pen.finish sees no target and resets. Actual React/controller reproduction: two-node Pen path, change the second Master combobox to m2, zero `doc.apply`, path no longer pending. Finish the old source before changing master/layer and guard delayed identity. Probe: `final-review-master.test.tsx` / `.config.mjs`, one bug-demonstration test passed.

## Verification and coverage

Read CONVENTIONS, binding product/editor specifications, P1 plan, active resume, acceptance record, current rulings and task reviews. Inspected document/editor handover, fields/history, tools, shaping identity/source bytes/order, text origin/controller lifecycle, bounded previews, metrics freshness, master/layer navigation, export/provenance, UFO writing, generated-artifact and licensing boundaries. Independently ran 159 existing tests across nine controller/session/text/panel/navigation files: all passed in 4.92 s. The additional probes expose gaps in those tests.

Candidate locale parity independently verified: 881/881 keys; separate translation and QA artifacts checked. Production diff whitespace check passed; whole diff flags only historical ledger Markdown whitespace/EOF polish. No generated API files committed except `.gitkeep`. Source-reference scan finds behavioural FontForge references, no adapted source; fixture/history licensing rulings remain unchanged.

Root reports integrated candidate check/lint/Prettier and 923 app tests passed; 453 Rust tests/fmt/clippy had passed with no subsequent Rust changes. These are attributed results, not independent reruns here. Latest browser drag verification initially outpaced transport settlement; root/T9 diagnosed a test synchronization defect (all applies and exact geometry eventually succeed). Its corrected explicit successful edit_end wait and final rerun remain required. Do not call final acceptance passed yet.

## Accepted limits / completion boundary

Untouched, verified, unambiguous original outlines/instructions may be copied; edits drop all hinting with warning. Model-path previews use shaped advances, not hinted compiled-font rasterization; 512-glyph/4096-pixel preview bounds, disabled kerning/vertical controls and explicit P2 backlog are accepted. Source Han export 58.697 s / 60 s passed narrowly; retain the qualification. API timing excludes transport; CPU synchronous painting excludes display scanout.

Root reports final embedded Tauri build at 6234825 passed in 45.87 s without launch. Earlier native open-dialog evidence is recorded, while remaining native workflow is paused/unverified under the user's input constraint. Reviewer performed no headed/native/global-input operations. One fix wave and scoped re-review remain. Finish P1, then stop; main merge is a separate user decision.
