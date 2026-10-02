# Independent hinter and atomic repair review

Read AGENTS.md, CONVENTIONS.md, p5f-protocol.md and implementation tasks.
Read-only implementation review; own only this file and
docs/p5f-w3-review-kernel.md. Review real ttfautohint bridge changes now as
w3_hinter completes verification. Then review w3_fix_all's atomic token-checked
edit cancellation API and bounded Fix All repair gesture when ready.
Check untrusted font/output validation, legitimate helper-glyph preservation,
glyph-index/cmap/metrics correctness, cancellation's undo/history isolation,
stale token and active-editor races, bounded convergence/cycle rejection and
locked/read-only validation. No weaker tests or undocumented loss accepted.
Send concrete defects early; verify fixes. Focused tests use shared Cargo flock
and own TMPDIR; no broad checks, implementation edits, native desktop actions,
commits or publication. Artifact check at start/end and cleanup owned scratch.

## Gap analysis

Real-binary coverage now exercises valid composite helper output, but the initial
count-only exception newly accepted malformed `maxp`/`loca` combinations. Review
must verify the narrowed helper identity and mapping checks using both authentic
output and forged-output regressions. Cancellation review checks kernel token
isolation and frontend handover barriers, then Fix All convergence and rollback
against focused tests and the real kernel once implementation is ready.
