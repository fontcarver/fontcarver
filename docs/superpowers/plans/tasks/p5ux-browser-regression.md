# UX repair browser regression

Date: 2026-10-02. User requested actual experience and repairs. Root is doing
agent-browser walkthroughs; disjoint agents are repairing shortcut/menu focus,
outline pointer lifecycle, bitmap cancel/pan and laptop workspace layout.

Own affected app/e2e specs and focused new UX regressions. Inspect coordinated
changes and update setup to explicitly open newly collapsed secondary controls,
preserving existing behavioral assertions. Add meaningful real-kernel/browser
coverage of repaired input behavior and 1366×768 tool accessibility where needed.
No weakening performance thresholds, retries or ignored cases. Wait for parent
source freeze before the final complete suite; use available prebuilt release
kernel/CLI, never compile Cargo implicitly. Coordinate ports with parent (root
uses 8861/1471) and other agents. Default suite's actual external CJK remains
enabled. Do not change product source or translations without coordination.

Record focused then full outcomes in docs/p5ux-browser-acceptance.md. Default
artifact budgets, task-owned scratch and cleanup apply. No desktop/global input
automation, commits or publication. Final installer build waits for UX acceptance.

## Gap analysis before implementation

The layout repair hides secondary kerning controls for a single glyph, so tests
that directly manage groups must open the Kerning disclosure explicitly. Text
proofs with a real gap still reveal the controls automatically. Browser coverage
will check the 1366×768 default canvas and keyboard access to the last tool,
menu focus and customized tool tap/hold, outline cancellation without an undo
entry, and bitmap cancellation/Space or middle-button pan without pixel writes.
These cases use real pointer/keyboard input and kernel state; only operating
system cancellation signals are dispatched where Playwright has no direct API.

## Completion

Implemented five browser regressions, adapted the explicit Kerning and Preview
settings entry points, and corrected subpixel pointer comparisons to a reviewed
0.01 font-unit tolerance while retaining edit/export/reopen checks. The first
complete run recorded 102 passes, two failures and one serial case not run;
the second failure exposed a real Bitmap editor entry-focus regression, repaired
by the keyboard owner and confirmed by the unchanged single-key test.

On frozen source `f6f6cf9`, the final complete suite passed **105/105**, with
zero retries and zero skips, including actual external CJK. All 8 ms performance
limits remain unchanged. See `docs/p5ux-browser-acceptance.md` for commands,
intermediate failures, final measurements and resource cleanup.
