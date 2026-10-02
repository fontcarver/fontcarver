# Editor workspace layout usability

Date: 2026-10-02. User asks for a modern, friendly editor and actual experience.
Parent observed the real browser at 1366×768: multiple always-visible kerning/
spacing rows above a single glyph squeeze the canvas to about 240 px; the lower
tools in the 17-item vertical tool strip are clipped. Evidence before changes:
.scratch/p5ux-experience/screenshots/editor-1366.png.

Read CONVENTIONS.md and binding docs/design/editor-ui.md. Own TextView.tsx,
TextView.module.css and related layout/UI tests; tool-strip layout/CSS may be
included after reporting exact files to parent (outline interaction owner must
not be overlapped). Inspect and fix the actual responsive workspace: preserve
canvas space, make every tool reachable on ordinary laptop heights, progressively
disclose secondary kerning controls while keeping active kerning workflows
discoverable and usable. Preserve user's panel preferences and existing editing
grammar. Keep neutral professional tokens, no new design system/dependencies or
marketing redesign. Do not rewrite functionality just for appearance.

Report a brief concrete design choice before editing. New English text keys may
be provided as a delta to parent; don't edit zh-CN, which is owned by independent
GPT-6.1 Sol translation. Coordinate en.json ownership before modifying it. Use
focused meaningful tests, type/lint/format, and parent browser before/after review.
Acceptance: docs/p5ux-layout-acceptance.md. No broad builds, no commits/publish,
owned scratch/process cleanup and artifact budgets required.

## Observed gap and chosen layout

The supplied 1366×768 screenshot shows the single-glyph editor's entire kerning
form above a canvas pinned to its 240px floor. The 17 tools shrink and then clip
instead of scrolling. Keep the 36px tool-strip design and all existing controls,
but group secondary kerning in a persistent disclosure, bound the secondary
control area's scroll height, and use nonshrinking 28px tool buttons with roving
keyboard focus and nearest-item scrolling.

Parent approved ownership of `TextWorkspaceControls.tsx`, its new CSS/tests,
`TextView.module.css`, `ToolPalette.tsx`, `toolPalette.module.css`, and its tests.
Only a newly selected canvas gap automatically reveals a closed kerning section;
ordinary model refreshes preserve the user's explicit choice. Existing child
panels remain mounted so folding the outer section does not discard drafts.

## Approved preview extension

Parent additionally approved `PreviewStrip.tsx`, its CSS/tests, a small extracted
`PreviewSettings.tsx`, a focused preview-layout test, and the TextView/PreviewWindow
test adaptations. An unset preview height now means 96px; saved heights survive.
The header settings disclosure temporarily raises the minimum to 160px without
writing the saved height. Closing it restores the original height and retains
draft settings. Root owns the new English key `proofPreview.settings` and its
separate GPT-6.1 Sol Chinese translation. UIStore and GlyphCanvas remain outside
this task's edits.
