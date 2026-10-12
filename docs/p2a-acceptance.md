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

| Check                                                                         | Result                                                            |
| ----------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| Rust: `fc-doc`, `fc-api`, `fc-model`, `fc-layout`, `fc-compile`, `fc-formats` | 414 passed, 0 failed                                              |
| `cargo clippy --workspace --all-targets -- -D warnings`                       | Passed                                                            |
| `cargo fmt --all -- --check` and `git diff --check`                           | Passed                                                            |
| `cargo run -p fc-api --bin gen_ts`                                            | Passed; generated bindings remain ignored                         |
| `pnpm check` and `pnpm lint`                                                  | Passed                                                            |
| `pnpm exec vitest run --maxWorkers=4`                                         | 114 files, 994 tests passed in 79.21 s                            |
| Prettier check on all changed/new app files                                   | Passed                                                            |
| `pnpm build`                                                                  | Passed; Vite retains its advisory for a bundle above 500 kB       |
| `pnpm exec playwright test --config e2e/p2a.config.ts`                        | 6 passed in 47.4 s                                                |
| Independent localization                                                      | 971 matching keys, matching interpolation, 7 focused tests passed |

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

Independent reviews: [frontend](#independent-frontend-review) and
[localization](#localization-qa) below. All reported blocking defects were fixed
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
FontForge comparison remain paused. The original P2a task did not commit,
merge or publish.

## Local checkpoint — 2026-09-17

The user subsequently authorized local commits. P2a is recorded in `918a994`,
following the existing rename/compatibility checkpoint `c338650`. Production
file hashes match the accepted workspace; this checkpoint adds no feature
changes. P2b proceeds through its separate
[design and implementation plan](superpowers/plans/2026-09-17-p2b-editable-layout.md).

## Appendix: earlier slice and independent reviews

These sections were separate files until 2026-09-24 and are merged here
unchanged apart from headings.

## Eligibility slice (2026-09-16)

Date: 2026-09-16. Branch: `p2a-kerning-resolution`; changes are uncommitted.
Scope: [bounded plan](superpowers/plans/2026-09-16-p2a-kerning-eligibility.md).

### Result

Implemented `fc_doc::kerning::check_horizontal_editing`: imported layout,
source-owned `kern`, and unparseable source produce distinct blocking reasons.
The parser handles prefixes, lookup references, conditional features, comments
and disabled blocks. Includes fail closed without reading files. Checks leave
source and stored data unchanged.

Moved existing FEA assembly into `Features::to_fea()`, shared with compilation.
The existing class-spelling regression moved with it. A new export/shaping test
covers prefix lookups, legacy/bracketed classes, contextual substitution and
disabled invalid source: both `AV` and substituted `AA` shape to advances
520 + 600 with the source pair value -80.

Dependency justification: `fc-doc` now directly uses the already locked
`fea-rs` 1.0.0 parser (`MIT/Apache-2.0`); no new external packages or version
changes. No FontForge source was used.

### Verification

| Command                                                                         | Result                                                  |
| ------------------------------------------------------------------------------- | ------------------------------------------------------- |
| Initial `cargo test -p fc-doc --test kerning_eligibility`                       | Expected failure: missing new interface                 |
| Same command after implementation                                               | 13 passed                                               |
| `cargo test -p fc-doc -p fc-model`                                              | 92 document + 37 model tests passed                     |
| `cargo test -p fc-compile --lib --test feature_source --test preserve`          | 4 unit + 1 source/shaping + 9 preservation tests passed |
| `cargo clippy -p fc-doc -p fc-model -p fc-compile --all-targets -- -D warnings` | Passed                                                  |
| `cargo fmt --all -- --check`                                                    | Passed                                                  |
| `git diff --check`                                                              | Passed                                                  |

Final diff reviewed for unchanged assembly, parser error handling, dependency
scope and preservation of pre-existing edits. No independent review was run.

### Boundary and next step

This checks layout ownership and syntax, not complete FEA compilation validity.
Future authoring queries and commands must enforce it. No API/UI or mutation
commands were added; P2a remains incomplete. Next: validated horizontal pair
set/remove, layout invalidation and undo/redo. Native automation, performance
investigation and FontForge comparison remain paused.

## Independent frontend review

Date: 2026-09-16. Scope: text/group authoring, gap identity, edit sessions,
query races, and transition architecture. Product fixes were made by their
authors; this reviewer changed only review documentation.

### Findings and disposition

- Group creation could replace an existing group's membership. The UI now uses
  the kernel's atomic create-only command; collision regression passes.
- Explicit-name Escape could commit through blur. A cancellation guard fixes
  this. Name drafts also preserve newer typing during asynchronous pair changes
  and retire when an external selection changes.
- Gap highlighting could disagree with the explicitly selected pair, and
  reselecting the same gap did nothing. Selection now checks actual pair names
  and submits every gap selection with request/context guards.
- Old group resolution could remain editable during generation refresh.
  Queries now include generation, and numeric commands verify captured group
  identities after preceding edits finish.
- Generation refresh initially dropped repeated keys and scope changes. Runs
  now retain captured targets through active/closing handover; scope intent is
  captured before draining edits. Delayed-generation key regressions pass.

No additional blocking finding remains in those modules. Preview geometry
continues to use kernel positions; gap rendering adds handles only. Group
queries and impact previews are bounded, with keyboard-accessible controls.

### Verification

From `app`, ran `pnpm exec vitest run` on `textGaps.test.ts`,
`kerningEdits.test.ts`, `useTextKerning.test.tsx`, `TextKerningPanel.test.tsx`,
`useTextKerningControls.test.tsx`, `kerningImpact.test.ts` (all under
`src/views/text/`), and
`src/views/glyph/panels/KerningGroupsPanel.test.tsx`.

```text
Test Files  7 passed (7)
     Tests  39 passed (39)
  Duration  23.21s
```

Shell tab-transition hardening is a separate active task. Its author was
alerted to edits started during an already pending navigation snapshot. Full
checks, headless persistence/export acceptance, and localization QA belong to
the milestone report; this review did not run native automation or benchmarks.

## Localization QA

Date: 2026-09-16.
Result: Chinese catalogue and static interface wording review pass.

The translation agent did not implement the feature or write its English copy.
Scope followed [P2a-04](superpowers/plans/tasks/p2a-04-localization.md).

### Reviewed content

- Translated all 87 keys under `kerningGroups`, `kerningText`, and `kerningCanvas`,
  plus two error summaries and the invalid-horizontal-kerning warning.
- Standardized kerning as `字偶距`, groups as `字偶距组`, and pair positions as
  `第一侧` / `第二侧`. Preserved the existing `母版` and `侧边距` terminology.
- Reviewed visible text, accessibility labels, tooltips, undo labels, loading and
  failure messages in the group panel, group side/details/pager components, text
  pair panel, and gap picker. Names and original kernel diagnostic details remain
  data rather than translated labels.
- Translated the final empty-inspector prompt explaining that selecting a glyph
  enables outline and kerning-group editing.
- Explicit zero and rule removal have separate explanations. Scope fallback copy
  now says it shows the resolved pair value: a more-specific exception is not
  incorrectly described as inherited by a group rule.
- Reviewed group membership and pair counts, shared-master effects, delete
  confirmation, imported-layout restrictions, and ambiguous-gap explanations.

### Verification

```text
pnpm exec prettier --check src/i18n/zh-CN.json
All matched files use Prettier code style!

pnpm exec vitest run src/i18n/i18n.test.ts src/i18n/errors.test.ts
Test Files  2 passed (2)
     Tests  7 passed (7)
```

A separate Node/i18next check found 971 English and 971 Chinese leaf keys, with
zero missing/extra keys and identical complete interpolation tokens, including
format hints. Runtime checks covered Chinese plural selection, counts, group-name
interpolation, side labels, explicit zero, scope fallback, group impact, and the
stale-document error.

During parallel implementation, broad checks found a test event missing `kind`,
React lint violations, and one not-yet-merged English key (946 tests passed).
These findings were sent to their owners; the final catalogue rerun above passed.
Final application-wide checks and headless visual verification are coordinated
separately in the P2a acceptance report. This review does not claim independent
screenshot or native desktop testing.

### Review fixes

Feature authors addressed untranslated local errors by using mapped API error
codes. The group creation path now rejects duplicate group names atomically,
after review found that the previous replacement command could clear members.
Transport-failure copy now asks the user to check stored membership instead of
claiming that no change occurred. Existing unrelated Chinese edits were retained.
