# Menu overflow at 200% zoom

2026-10-02. Scope: the user's English Help overflow repair, based on `cc03fca`.

At 1366×768 with actual browser zoom 200%, the CSS viewport is 683×384.
Originally Help ended at x=703.68 and its popup at x=901.54
([measurement](acceptance/p5ux/menu-200/before.json),
[before](acceptance/p5ux/menu-200/before-help-popup.png)). Now they end at
x=606.46 and x=675, respectively
([after](acceptance/p5ux/menu-200/help-en-light-200.png)).

The wordmark yields space at narrow widths; menu titles wrap when necessary.
Popup placement follows its title, clamps to the viewport and scrolls vertically
when needed. The shell and progress line follow the menu's height.

An isolated agent-browser session using Chrome 148.0.7778.56 passes
English/Chinese × light/dark at actual 200%
([zoom setting](acceptance/p5ux/menu-200/chrome-zoom-settings.txt),
[results](acceptance/p5ux/menu-200/browser-results.json)). All 11 titles and the
Help popup fit. Mouse activation, keyboard navigation, Escape/Tab, focus return
and resizing an open menu to 1024×768 pass; the
[narrow layout](acceptance/p5ux/menu-200/help-en-light-narrow.png) wraps correctly.
Zero browser JavaScript errors.

Verification:

- `pnpm --dir app check`, `lint`, `exec prettier --check .`, `test --maxWorkers=4`
  and `build`: PASS ([output](acceptance/p5ux/menu-200/frontend.log)).
  `Test Files 348 passed (348); Tests 2914 passed (2914)`.
- `playwright test e2e/menu-reflow.spec.ts e2e/welcome-reflow.spec.ts --retries=0`:
  `8 passed (17.1s)` ([output](acceptance/p5ux/menu-200/e2e-final.log)). Four new
  menu cases cover equivalent 683×384, narrower 512×384 and ordinary 1366×768
  CSS viewports. The original implementation failed the viewport assertion.
  An initial test focus assumption was corrected to honor the existing external
  opener behavior. The earlier complete 105-case milestone was not rerun.

No translation/dependency changes. Native high-DPI remains outside this check;
existing `f6f6cf9` installers predate both reflow repairs.

Owned browser/servers stopped; 28,867,727 bytes of task scratch and the private
TMPDIR removed. Resource check: **5.33 GiB build / 0.00 GiB temporary /
616.21 GiB free, PASS**. Work stops here.
