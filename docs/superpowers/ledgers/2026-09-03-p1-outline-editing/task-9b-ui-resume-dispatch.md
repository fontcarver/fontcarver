# T9b editor integration — Sol implementation, Astra review

Reviewed base: `b460871` on `p1-outline-editing` (T8 + scoped fix2 + preliminary
Chinese translation merged). Create new `p1-text-ui` worktree from this base.
Read CONVENTIONS, task-9b-dispatch.md, task-9b-resume-integration-rulings.md,
task-9-brief.md and binding editor-ui.md sections3/4/5/8/10/11/12. Historical
FontWorkshop/fw/path references are obsolete; use current fontpixel/fontcutter.

Complete the full Text tab integration, building on merged parsing/layout/
coordinator/compiled-identity foundation. The independent `p1-text-controls`
branch provides controlled TextBar and path-based PreviewStrip; inspect its
presentationTypes and coordinate with root, then merge its reviewed commit
when root identifies it. Reuse those components rather than duplicate them.

Ownership: TextView and text hooks/state/layout-to-render bridges; existing
GlyphView/Canvas/renderer/controller integration, uiStore/tabActions/TabContent/
status/menu/shortcut wiring; tests, text.spec.ts and required screenshots. New
files <=450lines; existing controller <=800 and extract responsibilities if
adding behavior. No Rust or dependencies expected. Preserve T8 handover gate,
LSB optimistic geometry, source/master identity and T7 guards.

The architecture rulings are binding, including stable tab identity, Unicode
titles/source mapping, compiled names resolved to document handles, glyph-local
command coordinates with run-origin viewport shifts, one active editor, stale
polling/cancellation/error feedback, zero-advance selection and visible-only
outline fetching for long CJK runs. Text size affects rendered scale. Fit runs
only on initial entry/explicitFit, not every click/edit. Keep existing single
glyph opening/stepping behavior and accessible canvas labels for existing tests.

Implement typing/focus/Ttool, activation, proper first Escape deselect/cancel and
second Escape to text mode. Flush all relevant pending gestures before changing
mode/target/master, including tool-owned transforms/unfinished drawing where
needed; controller.settle currently primarily handles core/aside/nudge. Shared
document barriers now cover disposal, but target changes still need proper tool
ownership. Suppress outline editing in text mode while preserving pan/zoom.

Metrics shortcuts use SetMetrics absolute accumulation +500ms EditSession, one
undo, shared controller gate, and flush on target/mode/master/othercommand. Do
not confuse Linux Control-as-Mod with100step; retain the platform ruling.
Kerning/vertical remain disabled with explanations under existing P2/P4 scope.

Statusbar activeglyph identity/metrics vs textlength/runcount, View-menu preview
toggle (no palette shortcut collision), shared master/features/direction and
independent striptheme. New en keys + English placeholders only; preserve
already-translated unrelated zh strings. Separate translator/QA follows.

Tests cover stabletabid/initiallegacytab, source mapping/RTL/repeats, clickthird
glyph then actual local-coordinate edit, Escape stages, master/size contexts,
stale-to-fresh withoutanotheredit, out-of-order async results, coalescedmetrics
undo and anothercommand. text.spec opensDejaVu/typesHamburgefonstiv/asserts
secondorigin==firstadvance/clickthird/edit/Escape twice. Existingacceptance e2e
branch adds realUIexportworkflows/perfgate; coordinate selectorchanges and
rerun afterintegration. Ports8794/1427, workers1, freshbinaries/config.

Fullcheck/lint/prettier/Vitest/e2e and agent-browser visualverification per
skill. Capture p1-text-runs/edit-in-place/preview-strip/stale screenshots with
honest captions. Report model-path preview limitation precisely. Commitfocused
milestones, keepworktreeclean atreport, no push/mainmerge/P2. Rootignoredledger
task-9b-report.md under300words with commits/tests/evidence/keys/limitations.
