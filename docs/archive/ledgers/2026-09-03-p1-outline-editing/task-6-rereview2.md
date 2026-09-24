# Task 6 — fix round 2 re-review (a7bf0c6..3007a90)

Read the 43 KB diff once; `editSession.ts`, `nudgeRun.ts`, `gestures/nudge.ts`, `gestures/drag.ts`
and `editorController.ts` were re-opened in the worktree because their hunks stop mid-class (the
`editSession.ts` hunk ends inside the `failed` getter). Suite not re-run. Eight probe tests were run
against the real modules from a scratchpad file with its own vitest config — the worktree is
untouched (`git status --porcelain` empty, HEAD 3007a90); each probe is named where it is cited.

### Finding Verdicts

**1 — a drag started during an open nudge run sent `edit_begin` before the run's pending
`edit_apply`** — ADDRESSED for three of the four ruled entry points. `EditSession` takes an `after`
gate whose `edit_begin` runs behind it (`app/src/editor/editSession.ts:54-63`, `:85-91`), the
controller keeps the closing run in `this.closing` (`editorController.ts:95-96`, `settle()` at
`:444-447`), and it is called by `pointerDown` (`:233`), by `runCommand` before its `doc.apply`
(`:380-382`) and by `setTool` (`:155`); the drag passes the gate through (`:570` →
`gestures/drag.ts:104-105`, `:296-300`). PROBE C — arrow then pointer-down in the same task, against
a transport that emulates `begin_edit`'s finalisation and `apply_in_edit`'s `InvalidEditToken`
(`crates/fw-doc/src/document.rs:139-142`, `:149-159`, `:163-168`) — gives
`edit_begin · edit_apply#101 · edit_end#101 · edit_begin · edit_apply#102 · edit_end#102` with no
error, i.e. the ruled test. PROBE F does the same for `toggleSmooth` (`doc.apply` last). Covered by
`editorController.test.ts:717-763`. The pointer path stays synchronous: `pointerDown` only `void`s
`settle()` (`:233`) and `pointerMove` previews from the local copy without awaiting (`:276-281`).
Two paths of the same ruling are *not* gated — the Info box commit and a new nudge run — both
reproduced under New Breakage.

**2 — proportional-nudge / Alt segment-reshape previews moved anchors the command never moves** —
ADDRESSED. The anchor branch is skipped whenever the gesture is in a per-point regime
(`gestures/drag.ts:470-476`: `this.deltas !== null || this.plan.anchors.length === 0`), which is
exactly when `command()` returns `perHandleCommand` before the `TransformSelection` branch
(`:587`, branch at `:606-616`). Regime by regime the preview set now equals the command set: one
delta → `MovePoints(plan.refs, handleMode)` + `TransformSelection(anchors, components)` vs
`translatePoints(movedRefs)` + translated anchors; per-handle (`perHandleDeltas`, `:407-420`) and
proportional (`nudgeDeltas`, `:427-454`) → one `MovePoints … PointsOnly` per key of the map vs
`translateEach` over the same map; mirror (`:617-627` vs `:462-470`) → `SetPointPosition` on the
opposite handle, which cannot coexist with `deltas`; transform-only → `TransformSelection` with the
anchors the preview moves. Tests `gestures/drag.test.ts:329-390` assert both directions (anchor
moved *and* a `TransformSelection` sent; anchor still, no `TransformSelection`).

**3 — the nudge preview was dropped before `edit_end` settled** — ADDRESSED. `NudgeCoalescer.end`
awaits `session.end()` and only then reports (`gestures/nudge.ts:118-125`), so the handover matches
`finishDrag` (`editorController.ts:616-629`). `session.end()` cannot reject — every step catches
(`editSession.ts:106-111`) — so a kernel failure still closes the run. Test
`editorController.test.ts:765-803` holds `edit_end` open, asserts the preview stands, releases and
asserts the hand-back plus the reload.

**4 — a run ended by a selection change re-based on stale contours** — ADDRESSED. The interrupted
run's result is carried forward (`nudgeRun.ts:80-95`) and consumed as the next run's base
(`:114`), and the late `onEnd` is dropped by a generation guard (`:98-109`, `:143-148`). Test
`editorController.test.ts:805-826` leaves both points moved; `:827-841` shows the second run still
closes with its own `edit_end`. Two caveats: the guard only covers the path that replaces the
coalescer (New Breakage, PROBE G), and the carried base assumes the interrupted run actually
committed — PROBE B shows it may not have.

**5 — the error reporter fired once per chained step when `edit_begin` failed** — NOT ADDRESSED.
`report()` does dedupe (`editSession.ts:119-123`), but the constructor's own catch calls
`this.onError` directly and never sets `errored` (`:92-96`), so the first `step` re-awaits the
rejected token and reports a second time. PROBE A (a real `EditSession` over a transport whose
`doc.edit_begin` throws, one queue + one `end()`) counts **2** `onError` calls, not 1 — three toasts
became two. The same line leaves `failed` false after a begin failure (`:134-136`), so
`finishDrag`'s force-refetch never fires for it (`editorController.ts:627`); harmless, because
nothing can have been applied, but it contradicts the getter's doc comment. No test covers this:
no test file fails `doc.edit_begin`, and the two tests the report names assert
`String(errors[0])`, not a count (`editorController.test.ts:483`, `gestures/drag.test.ts:620`).

**6 — dead code from the split** — ADDRESSED. `grep -rn "bandIsMarquee\|marqueeing\|actionRefs\|
CanvasPoint" app/src` → no hits; `selectedPointRefs` is still used in `editorActions.ts:155`, so the
import that `actionRefs` left behind is live.

**Key — `cmd.reason.noPointsSelected` for the point commands** — ADDRESSED. Nested under
`cmd.reason` in both bundles (`app/src/i18n/en.json:94`, `zh-CN.json:94` with the English
placeholder as ruled) and used by all four point commands (`views/glyph/glyphCommands.ts:178`,
`:188`, `:198`, `:212`); `glyph.open` (`shell/builtinCommands.ts:193`) and the two Font-tab commands
keep `noSelection`, correctly. The class-of-bug test resolves every registered command's
`disabledReason` through the app's i18next (`shell/builtinCommands.test.ts:53-64`).

### New Breakage in the Fix Diff

**Minor — a late close still clears a newer run's preview when the run is not replaced.** The
generation only advances when a *new coalescer* is built, which `NudgeRunner.press` does only on a
signature change (`app/src/editor/nudgeRun.ts:80-109`). A press with the *same* selection while the
previous run is closing reuses the coalescer, so the guard at `:145` matches and the late `onEnd`
runs: `close()` drops the run and calls `onEnd` (`:143-148`) → `closeNudge` clears the live preview
and requests a reload (`editorController.ts:453-458`), and `useLayerOutline.reload` refetches
regardless of `editing` (`views/glyph/useLayerOutline.ts:56`, `:83-93` defers only *events*). PROBE
G: with `edit_end` held open, the second run previews x = 210, and the moment the first run's
`edit_end` resolves the frame is back to `contours: null` with one reload requested mid-run. This is
new: before this round `onEnd` fired synchronously inside `end()`, so no close could ever land late.
Window is one kernel round trip after any `end()` — the 500 ms idle timer plus a further press is
the everyday way in. Self-heals on the next press.

**Minor — a drag that interrupts a run previews from the pre-nudge outline.** `startDrag` captures
`source.outline.contours` (`editorController.ts:567`) rather than `this.contours()` (`:488-490`),
and reloads are deferred for the run's length, so the run's delta is missing from the drag's base.
PROBE H: Ctrl+Right previews the node at x = 300, and pointer-down snaps it back to x = 230 for the
whole drag while the kernel will place it at 330. Pre-existing in shape, but the fix is what makes
it a real divergence: the run's last apply used to be dropped by the overtaking `edit_begin`, so the
document agreed with the stale preview; now it lands (the implementer's own interactive check —
930 + 1 + 123.93 — is this case). The hit test has the same base (`this.edit`, rebuilt only in
`setSource`, `:172`).

**Minor — the gate is not applied to the one gesture the diff rewrote: a new nudge run.**
`NudgeRunner.press` closes the old run with `void this.end()` and then constructs the next
`NudgeCoalescer` → `EditSession` with no `after` (`nudgeRun.ts:91-109`, `gestures/nudge.ts:90-97`),
and that constructor calls `edit_begin` synchronously (`editSession.ts:86-91`) while the closing
run's `edit_apply` is still one microtask away (`:106-111`, `:170-175`). PROBE B (Tab-then-arrow,
token-checking transport) gives `edit_begin · edit_begin · edit_apply#101 · edit_apply#102 ·
edit_end#101 · edit_end#102` and one `InvalidEditToken` reported — the same toast finding 1 was
about, on the same one-animation-frame window, and it also means the carried-forward base of
finding 4 promises a move the kernel dropped. `editorController.test.ts:827-841` covers this
sequence but asserts only the `edit_end` count, and the mock transport does not model token
invalidation, so it cannot see it.

**Minor — the Info box commit, named in the ruling, is not gated.** `commitField` calls `commit()`
straight to the kernel (`views/glyph/commitField.ts:22`), bypassing the controller and therefore
`settle()`; `Document::apply` finalises the active edit (`crates/fw-doc/src/document.rs:139-142`).
PROBE E2 (nudge, then a `commit` while the run's apply is queued) reports `InvalidEditToken`.
Reachability is far below the other three — it needs a field commit inside one animation frame of an
arrow press — but the ruled list is not complete without it.

**Minor — a second, different failure in a gesture is swallowed with no trace.** `report()` drops
every error after the first with no `console` fallback (`editSession.ts:119-123`), so an
`edit_end` that fails for a reason unrelated to the reported `edit_apply` disappears entirely. Scope
item (c) asked for this explicitly.

**Trivial — the gate never resets, so one unanswered call blocks all later gestures.** `settle()`
returns the stored `this.closing` even when no run is active (`editorController.ts:444-447`), and
`EditSession` waits on it before `edit_begin` (`editSession.ts:85-91`). Rejection is handled — the
gate calls `edit_begin` on both branches (`:87-90`) — and `NudgeRunner.end` catches everything
(`nudgeRun.ts:134-141`), so no error can deadlock it; only a transport call that never settles can,
and it then blocks every subsequent gesture rather than just its own.

### Out-of-Scope Observations

- Components are never previewed at all: `DragPreview` carries only contours and anchors
  (`gestures/drag.ts:263-267`) and the frame takes `components` from the source
  (`editorController.ts:216`), while `command()` does move them (`:606-616`). The mismatch finding 2
  fixed for anchors exists in the opposite direction for components, pre-existing.
- Parity for the per-point regimes was restored by dropping the anchor move rather than adding it,
  so a Ctrl+Alt drag on a node plus an anchor now leaves the anchor behind in both the preview and
  the document (`gestures/drag.ts:587`). Consistent, but a design call worth confirming.
- `NudgeCoalescer`'s own signature branch (`gestures/nudge.ts:83`) is now unreachable: the runner
  replaces the coalescer on any signature change (`nudgeRun.ts:80-95`), so a coalescer only ever
  sees one signature. The signature string is now built identically in two places (`nudge.ts:80-82`,
  `nudgeRun.ts:77-79`).
- `pointerDown` resets the view gestures but not an orphaned drag (`editorController.ts:234-236`);
  a drag that never saw its pointer-up leaves its `EditSession` open and is simply overwritten by
  the next `startDrag`. Pre-existing, and the new gate does not cover drag→drag (PROBE D2 is clean
  only because `end_edit` on a stale token is a silent no-op, `document.rs:181-190`).
- Report accuracy: the covering tests all exist, but every `editorController.ts` citation drifts —
  `settle()` given as `:406-420` (actual `:436-447`), `pointerDown` `:246` (`:233`), `runCommand`
  `:395-398` (`:382`), `setTool` `:139` (`:155`), the drag gate `:640` (`:570`); test ranges are
  ~10-16 lines low (`:701-750` for a describe at `:717`, `:753-793` for one at `:765`). The
  `glyphCommands.ts` and i18n citations are exact. The verification block paraphrases the runner
  rather than pasting it; the counts check out independently — 50 test files and 438 `it(` cases in
  `app/src`, matching "50 files, 438 tests (+8)". The finding-5 coverage claim ("covered by the
  existing rejecting `edit_apply` tests, which assert a single reported error") is not true of those
  tests, which assert only `errors[0]`'s message.

### Verdict

**Fix round:** Findings remain open — **5** (report once per gesture: a failing `edit_begin` still
raises two toasts, PROBE A). Findings 1, 2, 3, 4, 6 and the `noPointsSelected` key are addressed
with named covering tests, and finding 1's ruled scenario is verified against a kernel-faithful
transport. No Critical or Important breakage; six Minor/Trivial items introduced or left by the fix
are listed above, of which the two worth doing with finding 5 are the late close that clears a
same-selection run's preview and the ungated `edit_begin` of a new nudge run.
