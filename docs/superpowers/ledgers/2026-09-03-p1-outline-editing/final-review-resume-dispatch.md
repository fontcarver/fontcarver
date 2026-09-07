# Final whole-P1 review — Astra

Dispatch only after root identifies the final integrated candidate. Review the
entire P1 branch against `git merge-base main HEAD`, with special attention to
integration boundaries, rather than treating individually approved tasks as a
whole-branch approval. The current root is
`/home/chen/githubprojects/fontpixel/fontcutter`; historical ledger paths and
fw-* names predate the move and rename.

Read CONVENTIONS, the binding design/spec, the P1 plan, the active section of
the resume TODO, `docs/p1-acceptance.md`, current task reports/reviews and the
ledger's explicit rulings/deferred items. Use scoped existing tests or bounded
independent reproductions for suspected defects. Do not modify implementation,
merge, push, change the brand or start P2.

Review priorities:

- Document/session correctness and undo boundaries across tools, fields,
  gestures, source/tab/master changes and asynchronous cancellation. T8's
  controller replacement handover is a previous confirmed failure boundary.
- Text shaping names versus compiled GIDs/document handles; source-byte and
  visual order; repeated/zero-advance glyphs; local command coordinates after
  activation; master/source identity; stale/out-of-order responses and compile
  failures; one-controller lifecycle, editing state and two-stage Escape.
- Metrics keyboard coalescing and interaction with pending pointer edits;
  panel formula freshness, LSB geometry, custom transform origin and scoped
  guide indices/flags.
- Import/export preservation evidence, real edited/reopened outlines and
  shaping; large-font count/cmap; GUI warnings must reflect actual behavior.
- Key parity and separate translation/QA ownership, essential keyboard/accessibility
  behavior, error states and actual browser/desktop evidence.
- File limits, generated artifacts, transport boundaries and licensing. No new
  source should derive from unlicensed/copyleft reference code; historical
  removed fixture blobs remain a separate user-authorized history decision.

Binding P1 rulings are not new defects simply because the original ambitious
wording was broader: untouched verified original glyph set/order preserves
TrueType outlines/instructions verbatim; any outline edit drops ALL hinting and
warns. Contour-scoped path booleans, kerning, vertical text, export overlap and
the explicit P2 backlog remain deferred. Preview outlines may come from model
paths with kernel-shaped advances under the existing Task9b ruling; report this
accurately rather than claiming literal hinted compiled-font rasterization.

Acceptance claims must be supported by actually executed gates, not debug-mode
skips. Source Han release export passed narrowly at58.697s/60s, so retain that
qualification. Rust API timing excludes frontend transport; renderer-only
__fwPaint data does not establish pointer-handler-through-paint. Native dialogs
are separate from browser path prompts and desktop compilation.

Report confirmed blocking issues with severity, file/line, trigger, actual vs
expected behavior and evidence. Separate accepted limitations and optional
polish. One final fix wave and one scoped re-review follow; do not reopen
unrelated P2 scope. Write root ignored ledger final-review.md, concise findings
plus exact candidate/base commits and verification performed. Main merge is
the user's separate decision; P1 completion is the stop boundary.

Latest user constraint: desktop automation competed with the user's own input.
Native/headed/global input operations were stopped and the owned test process
closed. Review and any reproductions must use headless isolated browser/unit/
CLI checks only. Do not request interactive desktop actions to finish this run.
Record desktop build/launch/dialog evidence already achieved and remaining
native workflow as unverified due to the user's constraint. This does not waive
other P1 correctness or headless acceptance requirements.
