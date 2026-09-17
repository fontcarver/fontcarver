# P2a-05: headless authoring acceptance

## Scope and ownership

Own `app/e2e/p2a-kerning.spec.ts`, a scoped support module, and a scoped
Playwright configuration. Exercise the real debug kernel and browser UI on
isolated ports 1432/8799. Preserve all unrelated workspace changes.

Use temporary copies of the committed Minimal UFO fixture, with compiler-owned
kerning, visible A/V/a/v glyphs and 600-unit advances. A two-master designspace
variant covers master transitions. Never modify committed font fixtures.

## Interfaces

- Existing HTTP transport and document commands seed fixtures and inspect
  authoritative pair resolution and shaping.
- Glyph inspector controls expose accessible first/second-side labels.
- Text pair controls expose explicit glyph names, scope and numeric value.
- Canvas gap selection uses the actual pointer path; keyboard adjustments use
  the normal focused-canvas shortcut handler.

## Acceptance

- Create, assign, rename and explicitly remove an in-use group through the UI.
- Select a gap, edit group kerning, create/remove a zero exception and undo.
- Repeated shortcuts coalesce and preserve the existing numeric commit/cancel
  behavior; disabled `kern` restores the unkerned advance.
- Pending edits settle before save/export/reopen and master/document handover.
- A delayed failure retains the prior stored value and reports the failed edit.
- Imported binary layout stays previewable with kerning authoring disabled.
- Save and export artifacts reopen with matching authoritative shaping.

Run headless Chromium only. Native desktop automation, packaging, performance
investigation, and FontForge comparison remain paused. Record exact commands,
results and failure artifacts; product fixes belong to the implementing agents.

## Completed verification (2026-09-16)

The fresh debug kernel and isolated browser suite passed:

```text
cargo build -p fc-devserver
Finished `dev` profile [unoptimized + debuginfo] target(s) in 29.36s

cd app
pnpm exec playwright test --config e2e/p2a.config.ts
6 passed (47.4s)

pnpm exec tsc --project tsconfig.node.json
pnpm exec eslint e2e/p2a*.ts
pnpm exec prettier --check e2e/p2a*.ts
All matched files use Prettier code style!
```

All commands exited successfully. The six workflows cover actual canvas gap
selection, all four rule scopes and exception precedence, explicit zero/remove,
numeric cancel, shortcut coalescing and undo, source preview versus saved FCF
and reopened TTF shaping, shared-group create/assign/rename/remove with impact,
delayed master and document handovers, delayed save, failed edit recovery, and
read-only imported layout. UI tests observed and prompted fixes for text-input
focus, tab handover, and imported TTF preview preservation; implementing agents
own those product changes and their regression tests.

Artifacts retained by the successful run:

- `target/p2a-e2e-results/p2a-kerning-authors-groups-de40d-and-exports-matching-layout/text-kerning.png`
- `target/p2a-e2e-results/p2a-kerning-creates-assign-ea53e-oves-an-in-use-shared-group/group-impact.png`

The dedicated Vite config disables hot reload so concurrent checkout edits
cannot navigate a running acceptance page. Fixture files and server preference
directories are isolated; both owned servers exited after the run. Broad app
checks remain coordinated by the root task, avoiding concurrent repeated test
runs while cross-agent changes settle. No native desktop automation, timing
investigation, release package, or FontForge comparison was performed.
