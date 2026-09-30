# P5 CJK vertical inspector and existing-path verification

Owner: variable_cjk/cjk_vertical. Scope: checklist rows 19, 51, 110 and 111.

P4 already provides vertical advance/origin overrides, `vert`/`vrt2`
generation, CID mapping/order/CFF export, and top-to-bottom text layout.
This task adds missing top and bottom sidebearing fields to the glyph
inspector. A separate `VerticalMetricsPanel` uses the resolved outline bounds
and effective layer/master metrics. Editing TSB adjusts the origin while
retaining advance; editing BSB adjusts advance while retaining origin. Empty
outlines have no editable ink sidebearings. Layer override reset, explicit zero,
kernel validation and failed-write reconciliation remain available.

Files: new panel and focused component tests, a narrow extraction from
`MetricsPanel.tsx` coordinated with the horizontal metrics owner, English
`glyphView.metric` keys, user documentation and an acceptance report. No schema,
API, generated files, CID/export UI, text/proofing-owned files or audit edits.

Acceptance: first write component tests covering resolved metrics, both bearing
commands, inherited overrides, empty ink, invalid inputs and rejected writes;
then implement and run focused Vitest/ESLint. Re-run existing CID/vertical
document, API, compile and feature-generation evidence through the shared Cargo
lock if integration permits. Proofing owner verifies new proof surfaces; do not
claim every proof surface from an inspector change. Browser acceptance uses an
isolated headless session if the shared frontend/kernel is runnable. All scratch
lives under `.scratch/p5-cjk-vertical/`; check artifact budgets at boundaries.
