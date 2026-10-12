# Glyph workspace, Font Info and Export at 200% zoom

2026-10-04. User-selected local UI task, based on `56442b5`.

An isolated agent-browser session used Chrome 148's actual
[200% page zoom](acceptance/p5ux/workspace-200/chrome-zoom-settings.txt).
1366×768 and 1024×768 windows produced 683×384 and 512×384 CSS viewports;
`devicePixelRatio=2`, visual viewport scale 1 and CSS zoom 1.
All eight English/Chinese × light/dark × window-width combinations pass the
[interaction matrix](acceptance/p5ux/workspace-200/browser-results.json), with
zero browser JavaScript errors. The fixture is `DejaVuSansMin.woff2` (53 glyphs).

## Repairs and observed behavior

- The two fixed docks left only 135 CSS pixels for the editor at 683 pixels
  ([before](acceptance/p5ux/workspace-200/editor-before.png)). At widths up to
  900 CSS pixels, sidebar and Inspector now open individually in a temporary
  dialog. Close/Escape restores focus; widening restores saved dock visibility
  and widths. Explicit detaching still updates the saved docking state. Panel
  buttons stay reachable at the right edge of the tab strip
  ([sidebar](acceptance/p5ux/workspace-200/sidebar-en-light-narrow-200.png),
  [Inspector](acceptance/p5ux/workspace-200/inspector-en-light-narrow-200.png)).
- Font Info's previously hidden horizontal navigation
  ([before](acceptance/p5ux/workspace-200/fontinfo-before.png)) now wraps its
  actions, eight section tabs and search field
  ([English](acceptance/p5ux/workspace-200/fontinfo-en-light-200.png),
  [Chinese](acceptance/p5ux/workspace-200/fontinfo-zh-CN-dark-200.png)).
- The status bar scrolls locally and accepts keyboard focus, preventing canvas
  focus from shifting the whole application sideways at the narrower width.
- Applying metadata previously disabled the focused Apply button and lost
  keyboard focus. Focus now moves to Close after saving; Escape closes the
  dialog and returns focus to its opener. Reopening confirms the saved value.
- A single node's tall information box clipped its title and collapse control
  in a short canvas ([before](acceptance/p5ux/workspace-200/info-box-before.png)).
  The box now fits the canvas and scrolls only its fields
  ([English](acceptance/p5ux/workspace-200/editor-en-light-200.png),
  [Chinese](acceptance/p5ux/workspace-200/editor-zh-CN-dark-200.png)).

Real pointer dragging and undo pass at both widths. Keyboard editing, tool-strip
End navigation, panel Tab/Shift+Tab trapping and normal-width dock restoration
also pass. Export's six tested format choices and footer remain reachable;
actual TTF files exported through the UI pass CLI inspection with 53 glyphs
([English result](acceptance/p5ux/workspace-200/export-en-light-200.png),
[Chinese result](acceptance/p5ux/workspace-200/export-zh-CN-dark-200.png)).
Metadata and Export retain their existing scrollable bodies and fixed footers.
No strings, translation keys, dependencies or Rust behavior changed.

## Verification

The two new specs add eight cases at equivalent 200%, narrower and ordinary CSS
viewports. Their final run includes the existing welcome, menu, settings and
editor-input regressions. Each test owns its kernel/configuration/font sidecars;
the suite uses one worker and the prebuilt release CLI, with retries disabled.

```sh
FC_API_PORT=9878 FC_UI_PORT=1528 \
FC_E2E_CLI="$PWD/target/release/fontcarver" \
pnpm --dir app exec playwright test \
  e2e/workspace-reflow.spec.ts e2e/fontinfo-export-reflow.spec.ts \
  e2e/welcome-reflow.spec.ts e2e/menu-reflow.spec.ts \
  e2e/settings-reflow.spec.ts e2e/ux-input.spec.ts --retries=0
```

**23 passed (4.8m)**, zero failures, retries or skips
([final output](acceptance/p5ux/workspace-200/e2e-final.log)). The Vite proxy
connection-reset messages accompany the intentionally isolated kernel restarts;
the real-browser matrix separately records no browser JavaScript errors.

`pnpm --dir app check`, `lint`, `exec prettier --check .` and `build`: PASS.
`pnpm --dir app test --maxWorkers=4`: **348 files / 2,917 tests passed**
([frontend output](acceptance/p5ux/workspace-200/frontend.log)). The unit run
includes all behavior changes; the final node-info CSS was then covered by the
final browser matrix/regression and production build. The existing large-bundle
size warning remains. Documentation formatting, 130 local links/anchors and
whitespace checks pass.

During development, the original dock layout failed the width assertion.
Metadata testing exposed the real focus defect above; a subsequent test timing
race was corrected by waiting for Close to receive focus before pressing Escape.
The node-box test explicitly clears the previous multi-selection before selecting
one node. Browser driver corrections included integer pointer coordinates,
fresh fixture paths and waiting for the committed undo entry before undoing.
These intermediate attempts are not counted as passing runs.

Owned browser/servers stopped; private browser data and the remaining 6,712,819
bytes of task scratch were removed after retaining the evidence linked above.
Final resource check: **5.33 GiB build / 12,262 files, 0.00 GiB temporary /
2 files, 533.55 GiB free, PASS**.

The earlier full E2E suite, Rust suite, native high-DPI/trackpad behavior and Linux
installers were not rerun for this UI task. Larger zoom levels, long sessions
with large fonts and other specialized dialogs retain their existing backlog
status. The `f6f6cf9` installer checkpoint predates these source repairs.
The 112-row audit and broader project-effort estimate are unchanged.
