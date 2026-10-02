# Welcome and command palette at 200% zoom

2026-10-02. Scope: the user's next small task, based on `ea4ca7a`.

An isolated agent-browser session used Chrome for Testing 148.0.7778.56 with
its actual **Page zoom set to 200%**. A 1366×768 window reported a 683×384 CSS
viewport, devicePixelRatio 2, visualViewport.scale 1 and CSS zoom 1.
[Settings evidence](acceptance/p5ux/zoom-200/chrome-zoom-settings.txt) and
[four-combination results](acceptance/p5ux/zoom-200/browser-matrix.json)
distinguish this from pinch zoom or shrinking the viewport alone.

The English palette's long tool hints overflowed a 553-pixel list to 726 pixels;
six command labels had zero width. Other names were truncated, including
Close Tab. [Measurements](acceptance/p5ux/zoom-200/before.json),
[before](acceptance/p5ux/zoom-200/palette-en-light-before.png) and
[after](acceptance/p5ux/zoom-200/en-light-palette.png) retain the defect and fix.
Rows now wrap, unavailable reasons occupy a second line, and search/scope/footer
keep their height while results scroll. All 163 result rows fit horizontally.

English/Chinese × light/dark pass actual 200% checks: welcome Tab traversal,
visible focus, scrolling, palette first/last selection, Tab/Shift+Tab cycling,
Enter opening shortcut settings and Escape restoring the opener. No browser
JavaScript errors occurred. The
[ordinary 100% layout](acceptance/p5ux/zoom-200/palette-en-light-100.png) was also
visually checked.

Four new Playwright reflow cases use the equivalent CSS viewport. The
[original stylesheet fails](acceptance/p5ux/zoom-200/e2e-before.log) the overflow
assertion; the [corrected cases pass](acceptance/p5ux/zoom-200/e2e-final.log),
zero retries. Initial harness corrections handled fractional scroll positions
and Node-only TypeScript types; those attempts were not passes.

Frontend **2,914 tests / 348 files**, type checking, lint, Prettier and production
build pass ([output](acceptance/p5ux/zoom-200/frontend.log)). The earlier complete
105-case E2E milestone was not rerun or relabelled as 109 passing cases.

The English global menu's Help title extends to x=703.68 beyond the 683-pixel
viewport ([measurement](acceptance/p5ux/zoom-200/menu-followup.json)); this
separate shell issue is in [TODO](TODO.md). Hardware high-DPI, other dialogs and
larger zoom levels remain unverified. Existing `f6f6cf9` installers predate this
CSS change. No translation keys or dependencies changed.

The private browser and servers were closed; 37,839,420 bytes of task data and
the owned short browser TMPDIR were removed. Resource check: **5.33 GiB build /
0.00 GiB temporary / 616.31 GiB free, PASS**. Work stops after this task.
