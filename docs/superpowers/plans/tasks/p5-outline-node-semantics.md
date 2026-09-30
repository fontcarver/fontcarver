# P5 outline node semantics and modifiers

Scope: audit rows 22–27 and fill verified gaps in node modifier gestures, G2
smoothing, and path operations. Separate helper task plans bound G2 and path
operations. This plan owns `app/src/editor/gestures/drag/*`, narrow input
normalization in `GlyphCanvas.tsx`, and focused modifier regression tests.

Public interface: add optional physical `ctrl` to `Modifiers`; real browser
input always supplies it. Platform command `mod` remains the shortcut/nudge
multiplier. Explicit Ctrl suspends snapping and invokes Ctrl+Alt handle modes.

Acceptance:
- Macintosh Command and Control normalize independently; Linux Ctrl remains
  the platform command and physical control modifier.
- Ctrl alone suspends snapping, Command alone does not on Macintosh.
- Per-handle modifier changes in an active drag produce the same saved outline
  as its preview, including stationary pointer changes and multiple flushes.
- Existing proportional nudge and smooth preserve-outline behavior regresses
  cleanly in focused tests; parent runs global checks after integration.

No shared feature-audit/status edits, commits, broad formatting, or generated
API edits. Scratch: `.scratch/p5-outline-node-semantics/`.
