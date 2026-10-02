# Editor workspace layout acceptance

Date: 2026-10-02. Task: [p5ux-layout](superpowers/plans/tasks/p5ux-layout.md).

The initial 1366×768 browser capture showed a canvas around its 240px minimum,
an always-visible kerning form for a single glyph, and lower tools clipped from
the 17-tool strip. Changes follow the existing
[editor UI specification](design/editor-ui.md), retaining its neutral palette,
36px tool strip and compact controls.

## Result

- **Kerning on demand.** A keyboard-accessible Kerning disclosure contains the
  gap picker, pair editor, group outlines and kerning table. Single-glyph editing
  defaults to collapsed. New text runs with gaps show it unless the user chose
  otherwise. Selecting a new canvas gap reveals it; ordinary model refreshes do
  not reopen a manually closed section. The existing UI section preferences
  persist explicit choices. Child panels stay mounted, preserving drafts.
- **Canvas space.** Secondary controls scroll within a bounded area instead of
  consuming the entire editor. Existing proof/shaping/spacing entry points stay
  available. The outer editor retains its short-window scrolling fallback.
- **All tools reachable.** Buttons retain 28px height and the strip scrolls.
  Up/Down/Home/End move a single toolbar tab stop and scroll the focused tool into
  view. Enter activates; Tab/Shift+Tab leave the toolbar. External tool changes
  reveal the active item without stealing canvas focus.
- **Compact preview.** An unset preview height now means 96px. Saved heights
  remain intact. The Preview settings button reveals the existing controls and
  temporarily allows at least 160px without saving a different height; closing
  restores the original size. Draft settings survive disclosure changes. The
  resize grip, collapse, theme, flip and detach actions remain available.

Preview settings markup and validation were moved intact to `PreviewSettings`
to keep the strip readable. No new dependency or change to font data, editing
commands, metric arithmetic, or preview setting validation.

## Validation

With task-local absolute `TMPDIR`, the final focused command was:

```text
pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 \
  src/views/text/TextWorkspaceControls.test.tsx \
  src/views/glyph/ToolPalette.test.tsx \
  src/views/text/TextView.test.tsx \
  src/views/text/TextView.master.test.tsx \
  src/views/text/PreviewStrip.test.tsx \
  src/views/text/PreviewStrip.layout.test.tsx \
  src/views/text/PreviewWindow.test.tsx
Test Files  7 passed (7)
     Tests  37 passed (37)
  Duration  9.45s
```

`pnpm --dir app check`: exit 0 (`tsc -b`).
Scoped ESLint over every changed TS/TSX file: exit 0, no warnings.
Scoped Prettier: `All matched files use Prettier code style!`

Height tests cover an unset height, saved 120px, and saved 248px, including
temporary expansion, restoring the height, retaining a draft, and no height
persistence callback from toggling settings. Existing waterfall, blur, instance,
detached preview, live pen/master handover and text editing tests remain active.

The first preview batch exposed a pre-existing synthetic event omission in the
TextView test: its initial glyph selection sent pointer-down without pointer-up.
The new pointer lifecycle correctly treated it as a held pointer. In coordination
with the interaction owner, the test now sends the matching pointer-up; its
original two-Escape assertions remain unchanged and pass.

The browser regression owner independently reported its initial 1366×768 check
passed: canvas at least 300px, all 17 tools visible when focused, and Home/End/Tab
navigation. Final viewport, dual-theme and preview interaction evidence belongs
to the parent [experience acceptance](p5ux-experience-acceptance.md) and its
browser regression report; this document does not substitute component tests
for browser geometry checks.

## Ownership and cleanup

New English key: `proofPreview.settings`; root owns English insertion and the
independent GPT-6.1 Sol agent owns Chinese translation. This agent did not edit
either locale, UIStore, or GlyphCanvas.

Resource checkpoint: build 5.33 GiB / 12,257 files, temporary 0.13 GiB / 5,034
files, free 616.32 GiB; PASS. Task-owned scratch is removed before handoff; no
owned browser/server process was started. No commits or publication.
