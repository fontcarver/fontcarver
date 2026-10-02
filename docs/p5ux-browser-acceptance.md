# UX browser regression acceptance

Date: 2026-10-02. Task: `p5ux-browser-regression`.
Final application source: `f6f6cf9ce481d11314e25c540b5719fd0951a00f`.

The final complete default suite passed **105 tests**, with **zero failures,
zero skips and zero retries**, in **6.5 minutes**. The actual external CJK case
ran. The initial failed complete round is retained separately below.

The browser suite now covers the repaired input behavior through real
Playwright mouse/keyboard gestures and the existing release kernel. No Cargo
build, dependency change, retry or performance-threshold change is part of this
task. Operating-system `pointercancel` is dispatched explicitly because
Playwright does not expose a native mouse-cancel action; the gesture before it
uses real browser pointer events.

## Coverage and existing workflow updates

- At **1366×768**, a single-glyph workspace keeps at least 300 px of canvas,
  defaults secondary kerning controls closed, and reaches all 17 tools using
  ArrowDown, Home and End without clipping their buttons. Tab leaves the strip.
  Advanced glyph data remains explicitly accessible through both disclosures.
- The preview starts at **96 px**; opening settings temporarily provides at
  least **160 px**, closing restores the prior height, and collapse/expand stays
  accessible. Existing waterfall and named-instance tests explicitly open the
  new settings disclosure and retain all previous assertions. The existing
  single-glyph group-management test explicitly opens Kerning.
- The shortcut editor exposes exactly one Pen binding. Rebinding it to U allows
  hold-to-borrow and release-to-restore, ordinary tap selection, and typing in
  a text field. Menus accept keyboard entry after a pointer opening, Escape
  restores title focus, Tab exits, and a regular menu action restores canvas
  focus.
- Outline right-button Pen gestures leave contours/history untouched. Escape,
  pointercancel and blur cancel real node drags without an undo entry. A
  primary/right-button chord cancels as soon as primary is released; right-only
  motion cannot keep editing. A subsequent ordinary drag commits once and undo
  restores the original outline.
- Bitmap Escape, pointercancel and blur discard strokes with **zero
  `bitmap.apply` requests**. Saving and reopening proves canceled local pixels
  do not leak into the saved document. For a 64×64 bitmap at 32× zoom, middle and
  held-Space drags move the actual scroll position from `(100,100)` to
  `(160,150)`, preserving Pencil and every stored pixel.

## Validation

All commands use the same environment as this final invocation from the
repository root:

```sh
FC_API_PORT=8851 FC_UI_PORT=1461 \
FC_E2E_CLI="$PWD/target/release/fontcarver" \
FC_SOURCE_HAN_SANS=/home/chen/githubprojects/fontpixel/SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf \
TMPDIR="$PWD/.scratch/p5ux-browser-regression/tmp" \
pnpm --dir app exec playwright test --retries=0 \
  --output="$PWD/.scratch/p5ux-browser-regression/final-results"
```

Every Playwright invocation uses `--retries=0` and an explicit output directory
under this task's scratch directory. Bundled Chromium is unchanged.

| Command suffix after `pnpm --dir app exec playwright test`                                                                                                            | Outcome                                                                                                                                                        |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `e2e/ux-input.spec.ts e2e/bitmap.spec.ts --grep 'laptop canvas\|customizes one\|rejects right\|cancels interrupted\|pans a large'`                                    | Initial **3 passed / 2 failed**, 49.0 s. Both bitmap cases and laptop tool traversal passed. New test setup mistakes are described below.                      |
| `e2e/ux-input.spec.ts e2e/text.spec.ts e2e/p2a-kerning.spec.ts --grep 'laptop canvas\|customizes one\|rejects right\|waterfall as\|named instance\|creates, assigns'` | **5 passed / 1 failed**, 21.3 s. Kerning, both preview workflows, laptop preview and outline cancellation passed; a still-ambiguous menu selector failed.      |
| `e2e/ux-input.spec.ts --grep 'customizes one'`                                                                                                                        | **1 passed**, 5.6 s, after selecting the specific Select All Points command.                                                                                   |
| `e2e/ux-input.spec.ts --grep 'rejects right'`                                                                                                                         | **1 passed**, 6.1 s, including the final primary/right-button chord regression.                                                                                |
| `pnpm --dir app exec tsc -p tsconfig.node.json`                                                                                                                       | Passed.                                                                                                                                                        |
| Scoped ESLint and Prettier checks for the four changed E2E specs and coordinate helper                                                                                | Passed.                                                                                                                                                        |
| First complete default suite                                                                                                                                          | **102 passed / 2 failed / 1 serial case did not run**, 5.7 minutes. Performance and actual external CJK passed; failures investigated below.                   |
| `e2e/bitmap.spec.ts e2e/acceptance-workflows.spec.ts --grep 'every pixel tool\|P1 browser acceptance'`                                                                | **2 passed / 1 failed**, 19.4 s. Both pointer-edit/export/reopen workflows pass after the coordinate comparison repair. Bitmap entry focus failure reproduced. |
| `e2e/bitmap.spec.ts --grep 'every pixel tool\|cancels interrupted\|pans a large'`                                                                                     | **3 passed**, 9.5 s, after the bitmap entry-focus repair. The original shortcut test remains unchanged.                                                        |
| Final complete default suite                                                                                                                                          | **105 passed / 0 failed / 0 skipped**, **6.5 minutes**, zero retries, exit 0.                                                                                  |

Initial failures came from new test assumptions: the menu still called its zoom
command Larger Cells at that point; the first replacement selector matched both
Select All Glyphs and Select All Points. The new outline test also used a shared
helper's exact integer comparison after a compact-layout fit transform produced
a **0.00005 font-unit** subpixel difference. Its final assertion verifies the
actual intended movement with **0.01 font-unit** precision. These harness
failures are not evidence of product regressions or successful acceptance runs.

The first complete run exposed the same exact-equality assumption in the
existing `movePointWithPointer` helper: expected `(712,1300)`, actual
`(711.9998739482559,1300.0000630258721)`. The integration owner approved the same
**0.01 font-unit** coordinate tolerance for that helper. Both existing workflows
then passed, preserving actual edits, saved TTF inspection and reopened point
checks. The serial group's second workflow did not run in the failed complete
round; this was not an external CJK skip. All performance limits remain unchanged:
the first complete round recorded median **1.0 ms**, p95 **1.2 ms**, maximum
**2.8 ms**, pointer-down **4.0 ms**, and no samples above **8 ms**.

The other complete-suite failure is a product focus regression: after invoking
Bitmap editor from the menu, `document.activeElement` remains the closed menu's
`<button data-menu="bitmap">Bitmap</button>`, while the Bitmap editor tab is
active. The single-key dispatcher correctly preserves native interactive-control
keys, so M never reaches Select. A focused rerun reproduced this exact state.
The keyboard owner repaired initial canvas focus when entering from the body
or a closed menu title, preserving focus held by open menus or dialogs. All
three focused bitmap cases subsequently passed. The E2E does not manually
focus the canvas to conceal the broken menu entry behavior.

Final performance measurements stayed within the original 8 ms limit: median
**1.0 ms**, p95 **1.3 ms**, maximum **6.5 ms**, pointer-down **3.3 ms**,
renderer p95 **0.8 ms** and renderer maximum **2.4 ms**. No sample exceeded
8 ms. Exact final output excerpt:

```text
Running 105 tests using 1 worker
pointer-to-paint 300 points/200 frame-paced samples: median=1.000ms p95=1.300ms max=6.500ms over8=0 pointerDown=3.300ms rendererP95=0.800ms rendererMax=2.400ms
105 passed (6.5m)
```

## Resources and boundaries

Initial checkpoint: **5.33 GiB build**, **0.09 GiB temporary**, **616.40 GiB
free**, checker PASS. After focused validation: **5.33 GiB build**, **0.13 GiB
temporary**, **616.31 GiB free**, checker PASS. Immediately after the final
suite: **5.33 GiB build**, **0.28 GiB temporary**, **616.11 GiB free**, checker
PASS. The external licensed CJK font is reused without modification; native
desktop automation and package installation are outside this task.

Cleanup removed this task's **8.91 MiB / 723 files** of superseded browser
reports, logs and generated test data after retaining the acceptance evidence
above. Ports **8851 / 1461** have no listeners. The final shared-workspace
checkpoint is **5.33 GiB build**, **0.12 GiB temporary**, **616.26 GiB free**,
checker **PASS**. Shared dependencies, release binaries, external fonts and
other tasks' files were not touched. This browser task is complete and stopped.
