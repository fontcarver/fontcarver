# P2b implementation plan

Date: 2026-09-17. Status: tasks 01–11 complete within the documented static coverage.
Behavior: [P2b design](../specs/2026-09-17-p2b-editable-layout.md).

## Original planning checkpoint

The user accepted the next step of P2b design/task decomposition, then delegated
the decision to create local commits. This checkpoint delivers the audited
design, scoped implementation tasks and a safe starting order. It does not mark
any P2b production feature complete or resume the paused investigations.

The user subsequently authorized the next implementation step. Task 01 is now
implemented and verified: see [P2b-01 acceptance](../../p2b-01-acceptance.md).

The user has now authorized completion of all remaining P2b tasks, retaining
the earlier approval for delegated implementation, independent translation and
review, and local commits. All three gates below are complete; the acceptance
report records actual results and conservative conversion boundaries.

## Delivery sequence

| Task | Deliverable | Depends on | Primary owner/files |
| --- | --- | --- | --- |
| [01](tasks/p2b-01-source-assembly.md) | Shared mapped FEA assembly and UFO compatibility | P2a | `fc-model`, `fc-formats`, compiler/eligibility call sites |
| [02](tasks/p2b-02-draft-transactions.md) | Persisted drafts, feature snapshots, compile revisions | 01 | `fc-model`, `fc-doc`, FCF schema |
| [03](tasks/p2b-03-validation-api.md) | Diagnostics and guarded source activation API | 01, 02 | `fc-compile`, `fc-api`, trusted `fc-doc` apply path |
| [04](tasks/p2b-04-automatic-features.md) | Automatic source, freeze and writer ownership | 01, 03 | `fc-compile`, feature policy types |
| [05](tasks/p2b-05-layout-inventory.md) | Imported inventory, graph, limits, inspector queries | 01 | `fc-layout`, `fc-sfnt`, `fc-api` |
| [06](tasks/p2b-06-simple-decompiler.md) | Simple static GSUB/GPOS decompilation and comparison | 03, 05 | `fc-layout` and synthetic fixtures |
| [07](tasks/p2b-07-complete-static-decompiler.md) | Remaining static lookups, GDEF/BASE coverage | 06 | `fc-layout` |
| [08](tasks/p2b-08-conversion-and-lifting.md) | Atomic conversion/revert and proven kerning/anchor lift | 02–07 | `fc-doc`, `fc-layout`, `fc-compile`, `fc-api` |
| [09](tasks/p2b-09-features-editor.md) | Features/Lookups UI and generic edit handovers | 02–05 API contracts; integrate 08 | `app` shell/editor/features view |
| [10](tasks/p2b-10-localization.md) | Independent Chinese translation and terminology QA | 09 strings stable | `zh-CN.json`; translator owns it |
| [11](tasks/p2b-11-acceptance-review.md) | Corpus, headless acceptance and independent review | 01–10 | tests, fixtures and acceptance report |

Completed **tasks 01–11**; measured results and remaining format boundaries are
recorded in [P2b acceptance](../../p2b-acceptance.md). The next milestone is P2c
CFF export, followed by P2d formats.

The first regression for **task 01** loaded whole-file UFO FEA
beginning with `languagesystem` and a named class, compiled a ligature, exported
and reimported it, checking both source spelling and shaping. This exposed and
fixed the source-path inconsistency before adding a new UI. Tasks 02–11 then
added drafts, validation, generation, imported conversion, UI and acceptance.

Tasks 02 and 05 can proceed independently after 01. The frontend can build
against reviewed/generated contracts after 03/05; do not invent a competing
wire format. Tasks 04/06/07 share compiler/layout interfaces, so one coordinator
lands contract changes before other agents consume them. Within task 07, split
GSUB and GPOS implementations by module only after the common graph/writer
contract is stable. Translation must be a separate agent from feature authors.

## Gates and evidence

### Gate A — source authoring

Tasks 01–04 plus the source half of 09 prove invalid draft recovery, located
diagnostics, explicit successful apply, no-op/failure atomicity, undo, writer
ownership and save/export behavior. CodeMirror/transport races and IME are
part of this gate. Source authoring alone is not the P2b exit.

### Gate B — simple imported conversion

Tasks 05/06/08 plus imported UI establish the full read → inspect → prepare →
apply → edit → FCF save → reopen → revert workflow on a small licensed fixture.
Separately verify binary export/reopen shaping; binary files establish a new
import baseline and do not carry the original editing backup.
During development, unsupported structures keep conversion disabled. This is
a vertical implementation slice, not permission to drop the remaining lookups.

### Gate C — P2b complete

Task 07's static coverage matrix and task 11's acceptance must be complete.
Every claimed structure has a synthetic semantic test; unsupported structures
have rejection/no-mutation tests. Shaping samples supplement structural proof.
Real fonts include existing licensed fixtures and explicitly recorded corpus
coverage; an absent external fixture is reported as skipped, never as evidence
of broad coverage. Imported kerning includes at least one proven pair/group
lift workflow and a source-only fallback when lifting is not equivalent.
The same gate includes positive mark/mkmk anchor-lift and fallback fixtures.

## Validation commands and environment

Each task defines its focused failing tests before implementation. Run relevant
crate tests and Clippy, generated API checks for type changes, then app checks
for UI work. At the final gate:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p fc-model -p fc-doc -p fc-formats -p fc-compile -p fc-layout -p fc-api -- --skip source_han_export_meets_release_budget_when_available
pnpm --dir app gen:api
pnpm check
pnpm lint
pnpm --dir app exec vitest run --maxWorkers=4
pnpm build
```

Retain the exclusion above for the paused Source Han timing investigation.
Also run focused `fc-sfnt` import tests from task 05; a full sfnt suite, if needed,
excludes `source_han_sans_opens_within_the_release_budget`. Use a P2b-specific
isolated devserver/Vite/headless configuration,
not another agent's live server. Read the browser skill before visual work.
Update `docs/p2b-acceptance.md` only with measured implementation results.

Documentation planning checks: verify repository paths and local Markdown links,
task dependencies and API names, run `git diff --check`, and resolve independent
design-review findings. Production suites are not evidence that a plan is
implemented and need not be rerun for documentation alone.

## Local commits

Keep the completed P2a production tree intact. Three logical checkpoints:

1. Existing FontCarver rename, metadata and legacy-data compatibility changes.
2. P2a implementation, tests and acceptance records.
3. This P2b design, task files and current roadmap pointers.

The first tree is reconstructed in a disposable index and verified in an
isolated checkout. The P2a tree is checked against the previously accepted
working production files. No reset, stash, force push, remote publishing or
generated-type commit is needed. Record actual commit/check results at handoff.

## Review record

- [x] Read engineering conventions and binding P2/product scope.
- [x] Independently audit imported tables/compiler ownership and editor/source
  persistence; record BASE-only, legacy kern, JSTF and point-index dependencies.
- [x] Specify whole-file UFO normalization, draft/active separation, generic edit
  barriers, source-map encoding and compiler dependency direction.
- [x] Write scoped task contracts with explicit supported/blocked cases.
- [x] Complete independent final design review and resolve findings.
- [x] Validate document links and prepare all three local checkpoints.
- [x] Implement and verify P2b task 01 (224 Rust tests; see acceptance report).
- [x] Implement and verify P2b tasks 02–11; see the final acceptance report.

The complete static matrix and original-font rejection outcomes are recorded
explicitly; safe rejection is not reported as successful corpus conversion.

### Independent review resolution

The final reviewer identified five concrete gaps, now covered by the spec and
owning tasks: reject generic apply under imported authority; distinguish native
reopen/revert from binary reopen; guard clean imported source export before
touching output; implement bounded mark/mkmk anchor lifting with positive/fallback
fixtures; and bind diagnostics to effective compile-input revision. Corrected
the Features UI reference and required glyph-rendered lookup rules. The focused
recheck confirmed these fixes, with one Gate B wording correction applied above.

### Original planning checkpoint verification

- `c338650`: existing rename/compatibility checkpoint. The exact tree passed
  `cargo check --workspace --all-targets --locked`, 18 focused Rust tests
  (preferences, FCF, UFO and CLI), generated API/TypeScript checks and 20 uiStore
  tests in an isolated checkout.
- `918a994`: completed P2a checkpoint. All 762 existing production-file hashes
  matched the previously accepted workspace. Its recorded acceptance remains
  414 Rust, 994 frontend and 6 headless tests; no feature code changed during
  this planning/checkpoint work.
- At the original planning checkpoint P2b added documentation only. All 79 local
  links across 19 new/updated documents resolved, with task files 01–11 present.
  Implementation and acceptance have since completed as recorded above.
