# P2a horizontal kerning authoring acceptance

Date: 2026-09-16. P2a complete; no open blocking findings in this scope.

Scope is the [P2a design](superpowers/specs/2026-09-07-p2-kerning-design.md)
and [implementation plan](superpowers/plans/2026-09-16-p2a-authoring.md).
The user authorized full implementation plus independent implementation,
translation and review agents. Pre-existing workspace changes are retained.

## Delivered behavior

- Eligible source fonts support per-master horizontal glyph and group rules.
  Explicit zero overrides an inherited rule; removing a record restores the
  next matching rule. Imported-table and source-owned `kern` editing stays
  unavailable with a product explanation.
- Shared first/second-side groups support bounded search, create, assign,
  rename and explicit removal of an in-use group with its horizontal records.
  Create rejects duplicate names atomically. Commands synchronize membership,
  undo/redo and generation; layout-only edits preserve outline provenance.
- Typed queries expose eligibility, concrete resolution, glyph membership,
  bounded pair/group/member pages and diagnostics. Continuation pages reject
  stale generations. Source records are not expanded into group products.
- Text tabs expose exact glyph-name controls and unambiguous LTR canvas gap
  handles with a keyboard-accessible selector. Ligatures, mark clusters, RTL,
  unresolved glyphs and slash-name spans explain why their gaps are unavailable.
  The explicit pair panel remains available for eligible source fonts.
- Scope controls show the exact record, resolved value, supplying rule and
  bounded group membership impact. Enter/blur commits numeric edits; Escape
  cancels. Mod+Alt+Left/Right changes by 1, Shift by 10 and extra Control on
  macOS by 100, coalescing keys at the 500 ms idle boundary.
- Pair/master/document handovers and persistence drain captured edits. Late
  responses cannot replace the current pair. All visible spacing comes from
  kernel shaping; pending compilation is identified in the preview.

## Kernel evidence

Focused tests cover actual `AV` shaping with 600-unit advances: -80 gives
1120, disabling `kern` gives 1200 and removing the record restores 1200.
Overlapping group/glyph exceptions resolve and export as 0, -60, -40, -100.
Multiple masters, group renames, FCF and UFO round trips are checked semantically.

Invalid unresolved finite records survive source save/reopen and are excluded
from compilation with a warning. FCF cannot encode non-finite JSON numbers;
such a save now fails before touching the destination, preserving both the
previous file and in-memory data. Authoring commands reject non-finite values.

The exported-font reopen regression also verifies imported GPOS and GSUB
preview, a subsequent model-width edit, and FCF reopening. Layout preservation
checks glyph order before reusing tables; an unsafe order produces a diagnostic.

## Final verification

| Check | Result |
| --- | --- |
| Rust: `fc-doc`, `fc-api`, `fc-model`, `fc-layout`, `fc-compile`, `fc-formats` | 414 passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `cargo fmt --all -- --check` and `git diff --check` | Passed |
| `cargo run -p fc-api --bin gen_ts` | Passed; generated bindings remain ignored |
| `pnpm check` and `pnpm lint` | Passed |
| `pnpm exec vitest run --maxWorkers=4` | 114 files, 994 tests passed in 79.21 s |
| Prettier check on all changed/new app files | Passed |
| `pnpm build` | Passed; Vite retains its advisory for a bundle above 500 kB |
| `pnpm exec playwright test --config e2e/p2a.config.ts` | 6 passed in 47.4 s |
| Independent localization | 971 matching keys, matching interpolation, 7 focused tests passed |

Rust totals include generated binding tests. Two existing release tests remain
ignored, and `source_han_export_meets_release_budget_when_available` was
explicitly filtered to respect the paused performance work. The final imported
preview change was followed by full API/layout reruns. Exact commands and
per-crate totals are in the [kernel report](superpowers/plans/tasks/p2a-03-kernel-review.md).

The six browser workflows exercise actual pointer selection of a gap; all four
exception scopes; zero/remove/undo; feature disabling; numeric cancellation;
shortcut coalescing; group CRUD and deletion impact; delayed master/document
handover and save; rejected edits and recovery; FCF and TTF reopening; and
read-only imported layout. See the [headless report](superpowers/plans/tasks/p2a-05-headless-acceptance.md).
The [tab barrier tests](superpowers/plans/tasks/p2a-08-tab-barriers.md) additionally
cover new edit sessions and drafts created while an earlier transition waits.

Independent reviews: [frontend](p2a-frontend-review.md) and
[localization](p2a-localization-qa.md). All reported blocking defects were fixed
and covered by targeted regressions; the full frontend run above is the final
consolidated result.

## Visual acceptance

Used the `agent-browser` skill in an isolated headless session on UI/API ports
1433/8800, with its own configuration directory and generated source fixture.
Edited `AVAV` to -80 per `A/V` pair through the UI and verified the kernel's
2240-unit total, visible gap marker, and matching three preview sizes.
Inspected screenshots for English/light and Chinese/dark, including the
independent dark preview toggle. Controls and text fit without clipping.
Also inspected the headless group's member-count/deletion-impact screenshot.

Local artifacts (under ignored `target/`):

- [English light text editor](../target/p2a-visual/kerning-light-en.png).
- [Chinese dark text editor](../target/p2a-visual/kerning-dark-zh.png).
- [Group deletion impact](../target/p2a-e2e-results/p2a-kerning-creates-assign-ea53e-oves-an-in-use-shared-group/group-impact.png).
- [Consolidated frontend log](../target/p2a-app-tests.log).

The isolated browser and servers are closed after inspection. Tests did not
touch the user's running desktop session or configuration.

## Deliberate boundaries

P2b imported-layout authoring/FEA editing, P2c CFF export and P2d format expansion
remain later milestones. Horizontal operations retain vertical kerning.
Native desktop automation, release packaging, performance investigation and
FontForge comparison remain paused. This task does not commit, merge or publish.
