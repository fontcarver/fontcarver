# Final review F1 — persistence handover

Root Astra added `withSettledDocument`: capture the matching editor's tool/core/
metrics closure and previous document barrier before publishing persistence as
its own document barrier. Old tools finish before saving/exporting/closing;
subsequent edit sessions cannot start until persistence finishes. An unrelated
active document is not flushed. Failures release the barrier.

Save and ExportDialog use the boundary at the actual write. Explicit Discard
waits for existing work and closes without saving. Request Close reads the
kernel summary after pending edits finish, then asks about newly dirty work or
closes inside the same boundary; it does not rely on an event still in transit.

The initial helper incorrectly read a nonexistent EditorFrame.source field;
TypeScript and independent review rejected it. Corrected `8e6f383` adds an
explicit controller documentId and uses real controllers in all six persistence
regressions, including an unfinished two-node Pen path before Save.
 Regressions cover live metrics → Save → next metrics without token
loss, old/new edit sequencing around persistence, fresh dirty-close decisions,
explicit discard, unrelated editors and failure recovery. Corrected TypeScript, full ESLint and Prettier passed. Whole-branch reviewer will perform
the scoped re-review, followed by integrated tests.
