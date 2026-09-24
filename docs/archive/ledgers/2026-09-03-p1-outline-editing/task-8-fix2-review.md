# T8 fix2 scoped review — Root/Astra

Approved at `7849801`, covering `c951cfd`, `064d5b2` and `7849801` after P1
integration. This closes the confirmed findings in task-8-fix1-review.md.

Document-scoped handover gates replacement begin/apply/history calls while
cancellation keeps its own raw end/undo path. Completion removes the barrier;
documents stay independent. Live source changes cancel old pointer/metrics
ownership before installing new geometry; navigation still commits nudges.
Disposed callbacks cannot clear replacement state, while rollback errors remain
visible. Canvas Escape now takes the cancellation route before settlement.

Metrics resolution uses the current local event epoch as part of actionable
identity. Root caught and the implementer fixed a transient cross-document
generation comparison bug; a high-generation to low-generation document
regression now covers it. Stale results cannot keep Apply enabled.

Root independently ran109tests across session/controller/GlyphView/history/
resolution, plus focused disposal-error coverage and formatting. All passed.
The actual React pointer/key test verifies delayed begin/apply/end/undo followed
by the next command. The obsolete probe that manually calls the removed buggy
endNudge-before-Escape sequence is not a remaining product failure.

Implementer full suite:92files/869tests; check/lint/Prettier/APIgeneration pass;
e2e16pass/1optionalCJKskip because no release server was built. SourceHanfixture
is present; final release browser acceptance belongs to T12. Existing dark/narrow
screenshots were independently inspected in fix1. Controller800lines meets cap;
T7 guards/curve-format field retained. No new dependencies or licensing changes.
