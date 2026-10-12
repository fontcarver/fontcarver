# Independent editing fixes review

Read AGENTS.md, CONVENTIONS.md, p5f-protocol.md and the implementation tasks.
Read-only implementation review; own only this file and
docs/p5f-w3-review-editing.md. Review spacing shortcuts now (w3_spacing ready),
then bitmap native saving/vertical layout and CJK editing fixes when ready.
Check keyboard conflicts on all supported platform mappings, stale async state,
locks/read-only behavior, actual native/browser save delegation, correct metrics
and renderer bounds, evidence accuracy and user-visible translated strings.
Send actionable defects to parent and owner early and verify fix rounds.
Small focused tests may run in owned scratch with absolute TMPDIR and artifact
checks; no broad tests, source edits, desktop input, commits or publication.
Final report must separate source/test verification from unavailable native runs.

Parent-approved review extension: review the width-only `MetricsDrag` preview
arithmetic in `metricsHandles.ts` and affected gesture test contracts. Check
exact width/RSB and fixed ink before cancellation, then source restoration.
The kernel reviewer owns `doc.edit_cancel` and shared `EditSession` lifecycle.
