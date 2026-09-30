# P5-35: live active-layer outline audit

Scope: audit the active source layer continuously for open contours, coincident
stored points, and fractional coordinates. Provide canvas markers with hover
details, a keyboard-accessible issue list, per-test toggles, and explicit Fix /
Fix All for rounding fractional coordinates. Reuse existing atomic document
commands; do not guess a safe deletion for coincident points or change open-path
topology automatically. Keep larger outline-quality tests outside this slice.

Owned files: new `app/src/views/glyph/outlineAudit*`, `OutlineAudit.tsx`, its CSS
and tests, and a coordinated narrow mount in `GlyphCanvas.tsx`. Do not edit
renderGlyph, EditInspector, generated API bindings, translations or shared status.

Acceptance: focused tests cover issue locations, disabled tests, bounded output,
finite coordinates, safe single/batch fixes, hover details, disabled controls
during editing, failed mutation recovery and reload. The parent runs broad
frontend checks after integration. Browser verification requires an isolated
agent-browser session and must be reported honestly if deferred.

Resource checks: run `scripts/check_artifacts.py` before work and completion;
tests use `.scratch/p5-outline-audit/` as absolute TMPDIR. No Cargo builds needed.
