# P5 outline editor parity

Scope: verify audit rows 20–29, 31–36 and 89–91 against existing implementation,
then fill missing behavior. Node semantics and path operations are delegated to
`node_semantics`; annotations and path locking to `annotations_locking`.
Construction and image agents own linked guides, node links and image workflows.

This owner's first bounded change implements measurement deltas respecting the
italic angle, guide creation during a live measurement, lasso selection and an
eraser tool using the existing command/undo path. Existing tap/borrow, Info box,
HUD, background and audit behavior will be assessed with their focused tests.
New user-facing strings are English and independently translated.

Acceptance: targeted Vitest tests for event sequences, measurement spans,
italic-coordinate deltas and mid-drag guide commands; selection and erase command
behavior; frontend type/lint validation after integration; isolated agent-browser
headless verification. No FontForge implementation is copied. Root owns final
workspace checks and audit/status edits.
