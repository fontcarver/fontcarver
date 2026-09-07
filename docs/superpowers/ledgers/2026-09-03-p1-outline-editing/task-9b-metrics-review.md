# Text metrics runner review

Root reviewed `7c1ea7b` and `4a3c15b` against actual fc-doc SetMetrics semantics.
Nonempty outlines use accumulated absolute LSB and RSB values so scheduler
replacement and alternating keys preserve all changes. One EditSession and
idempotent end cover the 500 ms undo boundary; caller owns publication of its
closing promise into the controller/document barrier. Shortcut signs and
platform modifiers match the explicit integration ruling.

Initial review found empty-outline mismatch: the kernel's no-bounds RSB
assignment overrides LSB's width effect. Follow-up adds `hasOutline:false`:
blank glyph shortcuts adjust absolute advance width, report LSB 0/RSB width,
and never translate nonexistent geometry. Caller must derive outline presence
from actual flattened path. No Rust changes were needed.

Independent focused verification passed 10 tests after the follow-up. Approved
and merged; TextView/controller integration and actual one-undo browser behavior
remain the integration author's responsibility. No user-facing strings or new
dependencies were introduced.
