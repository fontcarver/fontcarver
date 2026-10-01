# P5 completion wave 2 checkpoint

Written 2026-10-01 by the integration agent as wave 2 closed. Its wave 1
counterpart is [the wave 1 checkpoint](p5f-wave1-checkpoint.md); the entry point
for the project is [STATUS](STATUS.md) and the remaining scope is
[TODO](TODO.md). Nothing is pushed and nothing is merged to `main`.

Plan: [`superpowers/plans/2026-09-30-p5-completion.md`](superpowers/plans/2026-09-30-p5-completion.md).
Shared rules for every agent: [`p5f-protocol.md`](superpowers/plans/tasks/p5f-protocol.md).

## Baseline

`a4b701d` (wave 1 code) and `976d213` (wave 1 docs), plus the seven `docs(p5)`
commits that recorded the wave 2 verdicts and the wave 2 integration task. The
resumed batch baseline before the waves is `6539b13` / `040609f`.

## The wave

| Task | Rows                                 | Review            | Fix round                                                                    |
| ---- | ------------------------------------ | ----------------- | ---------------------------------------------------------------------------- |
| g    | 64–71, 101–103                       | accept with fixes | 6 items (SetStrike lock scope, unencoded waterfall, retained check script)   |
| h    | 73–76, 95, 109, 111 (proofing)       | accept with fixes | 5 items (range truncation, TSV escaping, keyed refusals, three-state `dlig`) |
| j    | 82, 83, 85, 86, 87, 92, 93, 94, 97   | accept with fixes | 7 items (kernel-enforced guide locks, handle guards, stroker band 1.0)       |
| k    | 61, 98, TrueType scope               | accept with fixes | 7 items (CFF2 blend basis, flex mask offsets, 16 CLI `--hint-…` flags)       |
| l    | 19, 110, 111 (Edit View/`VORG`), 112 | accept with fixes | 5 items plus a `VerticalMetricsPanel` stale-override defect its e2e found    |
| m    | 80, 107, 108                         | accept with fixes | 5 items (quick-help button reachable, spelling, a11y guards, `--bm-*` test)  |

Every row of all six tasks is **Complete** in
[the audit](p5-feature-audit.md), several with the limits their acceptance
documents state: `docs/p5f-{g,h,j,k,l,m}-*-acceptance.md` carry the commands,
the browser evidence, the independent FontTools/FreeType/HarfBuzz checks and
those limits. Two zh-CN translation passes followed the wave
([1](p5f-localization-wave2-pass1.md), [2](p5f-localization-wave2-pass2.md)).

## Integrated verification

Run once, in this order, on this checkout:

```text
pnpm --dir app gen:api                                              exit 0
cargo fmt --all && cargo fmt --all --check                          exit 0
cargo clippy --workspace --all-targets -- -D warnings               exit 0
cargo test --workspace --no-fail-fast    2,086 passed, 0 failed, 2 ignored (277 binaries)
pnpm --dir app check (tsc -b)                                       exit 0
pnpm --dir app lint (eslint .)                                      exit 0
pnpm --dir app exec prettier --check .                              exit 0
pnpm --dir app exec vitest run           2,774 passed in 339 files, 0 failed
pnpm --dir app build                                                exit 0
pnpm --dir app exec vitest run (again, after the fixes below)
                                         2,775 passed in 339 files, 0 failed
FC_API_PORT=8787 FC_UI_PORT=1420 playwright test
                                         91 passed, 3 failed, 1 skipped of 95
playwright test --config e2e/p2a.config.ts                        11 passed
playwright test --config e2e/p2b.config.ts                        17 passed
playwright test --config e2e/p2cd.config.ts                        8 passed
```

The default Playwright configuration was run twice, before and after the fixes,
and passed 91 of the 94 specs that execute each time — on a different three.
`app.spec.ts:217` and `p2a-kerning.spec.ts:546` failed in the first run, are
fixed below, and pass in a targeted rerun; `p2b-imported.spec.ts:75`,
`fontview.spec.ts:303`, `panels.spec.ts:108` and `p2b-source.spec.ts:26` each
failed in one run and passed in the other, and the P2b pair passes 17/17 in its
own configuration. They are the shared-document and virtualisation timing
failures [TODO](TODO.md) records for that configuration.

## Integration fixes made here

- `app/src/views/text/proofStepping.ts`: the proof stepper claimed `↑`/`↓`
  anywhere in the Text view that was not a text field, including the preview
  strip's focusable resize separator, which moves on those keys itself — so
  keyboard resizing silently did nothing (`app.spec.ts:217`).
  `canStepFromSurface` now also refuses an interactive target, using the shell's
  existing `isInteractiveTarget` (which excludes the canvas, so canvas stepping
  is unaffected). A new case in `proofStepping.test.ts` holds both halves, and
  `docs/user/proofing.md` states the rule.
- `app/src/views/text/KerningTable.test.tsx` and `app/e2e/p2a-kerning.spec.ts`:
  task m's "Class" → "Group" wording.
- `app/src/App.test.tsx`: the File menu's disabled rows no longer carry a native
  `title` — Quick Help owns hovering a row — so the reason is asserted through
  `aria-description`, which is what `MenuItem` sets, beside the inline hint the
  same case already checked. `MenuItem`'s doc comment said "tooltip" and now
  says what the component does.
- `app/src/views/glyph/useFontEditorSettings.test.ts`: the last assertion read
  the hook's value in the same tick as the refetch it had just triggered. The
  hook deliberately reports nothing until the new generation answers, which is
  one microtask later, so the value is now awaited. Proved with a probe run
  before and after.
- The audit's wave 1 note about the intermittent `text.spec.ts:122` failure is
  replaced by task h's diagnosis (the cold first `text.shape` compile, about
  four seconds, not dropped input).

## Commits

`9b72af8` (code, all non-docs paths) and the `docs(p5)` commit that follows it
with the records below. Nothing is pushed and nothing is merged to `main`.

## Records updated

[The audit](p5-feature-audit.md) (35 rows, header and counts: **110 Complete, 2
Partial, 0 Pending verification** of 112, from 75 / 5 / 32),
[STATUS](STATUS.md), [TODO](TODO.md) (wave 3 scope),
[the estimate](progress-estimate.md) (dated reassessment, **about 77%, range
70–85%**, weighted band 70.7–83.8%) and the plan's wave table.

## What wave 3 inherits

Rows 9 and 62 only: no macOS or Windows binary of this code exists, the AppImage
bundles 73 copyleft-licensed shared objects with notices for 10, and the deb's
install has been verified by extraction rather than `apt install`. Beyond the
rows: a release rebuild from this code, the English wording items the wave 2
translation passes flagged (with a zh-CN delta pass after them), the documented
limits each acceptance document states, and the final row-by-row gate. All of it
is in [TODO](TODO.md).

## Resources

`python3 scripts/check_artifacts.py` at the start of this pass: build 9.14 GiB /
17,356 files, PASS. The workspace test run took it to 21.47 GiB / 20,834 files,
over the 20 GiB ceiling, so the per-member `cargo clean -p …` for all fifteen
members ran between Cargo invocations (15.4 GiB removed, back to 6.27 GiB) and
again at the end. No other task's cache, scratch or evidence was touched.
