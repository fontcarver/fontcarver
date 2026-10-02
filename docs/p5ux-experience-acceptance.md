# Editor experience review and repair

Date: 2026-10-02. Actual browser walkthrough complete; final integrated results
are recorded in [browser acceptance](p5ux-browser-acceptance.md) and
[integrated acceptance](p5f-w3-integration-acceptance.md).

The user requested actual usability evaluation, modern editor quality, rich
shortcuts and expected mouse interactions. The private headless agent-browser
session `fcux-root-20261002` used the real release kernel on 8861 and Vite on
1471, isolated configuration, a new Basic Latin project and a copied DejaVu Sans
fixture. No personal desktop input or original font was modified. The binding
design remains the neutral, canvas-focused professional editor.

## Reproduced problems and repairs

**UX-001: short-window controls hid tools.** At 1366×768, expanded kerning rows
and the preview left roughly 240 px for the canvas; the 17-tool palette was
clipped. [Before](acceptance/p5ux/editor-1366.png). Secondary controls are now
bounded and scrollable, single-glyph kerning starts collapsed, and the preview
starts at 96 px with settings disclosed on demand. Existing height preferences
are preserved. All tools are reachable by scrolling or Arrow/Home/End; Tab exits.
[Chinese dark workspace](acceptance/p5ux/dejavu-edit-dark-ready-1366.png),
[last tool reached with End](acceptance/p5ux/tools-end-dark-1366.png),
[preview settings](acceptance/p5ux/preview-settings-dark-1366.png).

Advanced glyph database/custom JSON controls now start collapsed, remember
explicit disclosure preferences and keep drafts mounted. Panel toggles use the
existing Lucide icon family. Zoom commands use consistent Zoom In/Out wording.
The [layout acceptance](p5ux-layout-acceptance.md) and browser regressions verify
these behaviors; the database panel has nine passing focused cases.

**UX-002: right-button dragging changed outlines.** On a newly drawn rectangle,
right-dragging its top-left node changed the shape and recorded an edit:
[before](acceptance/p5ux/right-drag-before.png),
[after](acceptance/p5ux/right-drag-after.png),
[reproduction video](acceptance/p5ux/right-drag-before.webm).
Secondary-button input is now rejected; primary/middle input is owned for the
whole gesture and interruption cancels atomically. Actual retest on DejaVu A
leaves the outline and Saved status unchanged:
[fixed](acceptance/p5ux/right-drag-fixed.png),
[retest video](acceptance/p5ux/right-drag-after.webm).
[Pointer acceptance](p5ux-pointer-review.md) and
[independent review](p5ux-outline-review.md) also cover asynchronous cancellation,
Space release/focus loss, normal Pen multiple clicks and mixed-button release.

Related [bitmap repairs](p5ux-bitmap-input-acceptance.md) make interrupted
strokes cancel safely, seal a gesture before saving and retain middle/Space
panning. The [independent review](p5ux-bitmap-review.md) exercised save/selection
races. [Keyboard repairs](p5ux-shortcuts-review.md) preserve remapped tap/hold
tools, remove duplicate settings rows, protect IME confirmation, restore menu
focus and distinguish physical Control from portable Mod. Synthetic modifier
tests do not establish native macOS/Windows parity.

The complete browser suite subsequently caught a menu-entry regression: a newly
opened bitmap editor left focus on the closed menu title, disabling immediate
letter shortcuts. The [entry-focus repair](p5ux-bitmap-entry-focus-acceptance.md)
hands focus to the loaded canvas for pointer and keyboard menu entry, while
respecting fields, other buttons, open menus and same-commit modal dialogs.
The original browser shortcut test now passes without a test-only focus action.

## Actual walkthrough

Creating a Basic Latin font, opening A, drawing a rectangle and Undo were
exercised through UI controls. The revised layout was inspected in Chinese/dark
at 1366×768 and [English/light at 1440×900](acceptance/p5ux/dejavu-edit-light-1440.png).
End reached the last tool; Home returned to the first. Opening preview settings
changed the accessible strip height from 96 to 160; closing restored 96.
The [AVATAR proof](acceptance/p5ux/proof-ready-light-1440.png) rendered the real
font, kerning and waterfall. Save As wrote a copied FCF project. TTF export
reported **752,720 bytes, 6,253 glyphs**, with the existing explicit warning that
Macintosh name records are replaced with Windows Unicode records.
[Export dialog after completion](acceptance/p5ux/export-complete.png);
[Result panel text](acceptance/p5ux/export-result.txt).
Independent fontTools parsing confirmed the glyph count and A cmap entry;
[reopening the exported font](acceptance/p5ux/reopened-export-ready.png) rendered
A successfully. No browser JavaScript errors were reported.

A CLI wait for the nonexistent text “Exported” timed out although export had
succeeded; the actual Result panel and output file established completion.
During development, HMR occasionally remounted the workspace and showed recovery;
that development-server behavior was not counted as a product defect.

## Assessment and scope

The editor has substantial everyday editing and shortcut coverage. This review
found real safety and space-use problems and repaired them; the revised default
workspace is more usable on an ordinary laptop. These results do not establish
perfect usability, every device gesture or professional long-session comfort.
Trackpad, high-DPI/text scaling, long large-font sessions and designer trials,
plus broader feature expansion, are explicitly in the [future TODO](TODO.md#future-experience-and-feature-backlog).
After this walkthrough, the user authorized one additional small
[welcome-discovery task](p5ux-start-discovery-browser-acceptance.md), which is
also complete. The broader future backlog remains unimplemented.

The [independent GPT-6.1 Sol delta review](p5ux-translation-acceptance.md) left
3,944 matching English/Chinese keys at this UX checkpoint; the separate welcome
delta increases the final catalogues to **3,945 matching keys**. Final frontend/E2E
counts and source-matched installer evidence are in
[integrated acceptance](p5f-w3-integration-acceptance.md); earlier test failures
remain described separately. Screenshots/videos are retained under
`docs/acceptance/p5ux/`; copied fonts and browser/server data are disposable.

Root walkthrough cleanup completed: the private browser and both owned servers
were stopped; 27 MiB of copied fonts, intermediate captures and temporary data
were removed. Resource check passed at **5.33 GiB build / 0.06 GiB temporary /
616.42 GiB free**. Other tasks retain only their still-active verification data.
