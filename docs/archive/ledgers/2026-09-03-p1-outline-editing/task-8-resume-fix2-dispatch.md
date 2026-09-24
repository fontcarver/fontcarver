# T8 fix round 2 — Sol implementation, Astra review

Worktree: `/home/chen/githubprojects/fontpixel/fontcarver/.worktrees/p1-t8`,
branch `p1-t8` at `112ab38`. Read CONVENTIONS, original task-8 dispatch and
`task-8-fix1-review.md`. The prior implementer cannot resume because the tool
reports an agent thread limit; its committed work and reports remain intact.

Fix the three confirmed review findings, with actual integration regressions:

1. Preserve cancellation ordering through controller disposal/remount and live
   source identity changes. An old cancellation must finish before any new
   begin/apply/undo for the same document. Use a small document-scoped handover
   barrier or equivalent, clean it on completion, and keep documents independent.
   Capture the old document before replacing geometry. Route replacement edit
   sessions, field commands and shell history through the same barrier where
   needed. Cancellation's own end/undo must not await itself. Clear old live
   aside ownership immediately; same-source reload must not cancel an edit.
   Check outline drag/nudge disposal too, since they share this protocol.
2. Canvas Escape must cancel the current gesture before generic settlement can
   finish it. Test the real canvas event route, after at least one applied frame,
   including a delayed closure and subsequent command. Preserve ordinary nudge
   closure and two-stage selection behavior needed by T9b.
3. Include current metrics/document generation in actionable resolution identity.
   A referenced glyph's change must disable Apply while new resolution is
   pending, even when target and formula text remain identical. Test the actual
   event-triggered refresh and out-of-order result handling.

Avoid widening into unrelated P2 features. Keep T7 finishDrag preview guard and
curve-format identity. Production controller <=800 lines; extract by coherent
responsibility. Keep existing user-facing design. No generated API commits or
new dependencies expected.

After fixes are clean, merge reviewed `p1-outline-editing` (at least `3716ab7`)
into the worktree to include portable fixtures and compiled glyph identity.
Use fresh local API generator/binaries. Run targeted regressions, check/lint/
prettier and full Vitest. Browser regression for actual Escape/source change
where feasible; e2e workers=1, own ports8793/1426. If optional CJK now runs,
build release server or explicitly isolate it for separately owned T12 evidence.

Commit focused changes; no push or main merge. Root ledger report
`task-8-fix2-report.md` under300words; report exact checks and limitations.
