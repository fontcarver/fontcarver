# Mouse and canvas usability audit and repair

Date: 2026-10-02. User explicitly asks to try the app, assess modern UI/UX and
whether expected mouse events exist, and improve concrete usability problems.

Read CONVENTIONS.md and docs/design/editor-ui.md. Initially read-only: trace
pointer selection, marquee, double click, drag, middle/Space pan, wheel/pinch,
right-click menus, modifiers, escape/cancel, pointer capture/lost focus and
bitmap/text differences. Check reachable UI and meaningful existing tests;
report concrete gaps and priorities rather than assuming every desirable
gesture must be implemented. Parent owns actual browser walkthrough and visual
changes. Request a disjoint repair assignment before editing. No broad builds,
no desktop/global input automation, no commits or publication.

Record findings in docs/p5ux-pointer-review.md. Respect artifact budgets and
task-owned scratch. Preserve intentional platform/feature limits; new English
UI text needs the independent translation agent.

## Authorized outline repair scope

Parent assigns the demonstrated outline right-button mutation and Space/focus
lifecycle defects, plus pointer cancellation semantics. Own GlyphCanvas,
editorController and related outline gesture/tests. Secondary buttons must not
start editing; preserve legitimate middle/Space pan. Escape, pointercancel,
lost capture and blur must have intentional tested cancellation/cleanup behavior
without stale gestures or unintended commits. The separate bitmap task owns
bitmap interactions. The layout task owns TextView/ToolStrip layout and styling;
coordinate any overlap before edits. No speculative context-menu redesign is
required; report its current scope clearly.

Coordinator follow-up: diagnose the three `GlyphView.test.tsx` failures from
the final full Vitest run; repair real input fixtures without weakening
assertions, or product code only if an actual integration defect is found.
Run that file and the related pointer/controller suites; keep the browser
source freeze when possible. No commits.
