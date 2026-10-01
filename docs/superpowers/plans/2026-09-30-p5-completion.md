# P5 completion plan

Status: active from 2026-09-30. Goal: close every Must-have and Should-have
row of `docs/p5-feature-audit.md` (the P5 exit gate), finish the release block,
and leave the repository verified, documented and committed on
`p5-color-authoring`.

The orchestrator (this plan's owner) only plans, dispatches and integrates.
Implementation, testing and acceptance are done by delegated implementation
agents; translation by separate translation agents. The shared rules are in
[`tasks/p5f-protocol.md`](tasks/p5f-protocol.md); every task file references it.

## Starting point

- The resumed 2026-09-30 batch is committed as the baseline (see `git log`).
  Its verification: 289 files / 2,233 frontend tests, 1,710 Rust passes,
  Clippy and formatting clean, production build passing.
- Audit at start: 3 Complete, 3 Partial, 106 Pending verification. The
  [resume audit](../../p5-resume-audit.md) classifies the pending rows as
  13 reusable-evidence, 76 partial and 20 not-implemented-at-resume; many of
  the latter received implementations in the batch that were never accepted.

## Waves

Each wave: parallel area agents on the shared checkout with disjoint file
ownership, then a translation pass (zh-CN for the wave's new keys), then an
integration pass (full Rust and frontend verification, regressions fixed,
audit rows updated, commits). Areas that touch the same directories are placed
in different waves.

| Wave | Task file                     | Area                                     | Rows                               |
| ---- | ----------------------------- | ---------------------------------------- | ---------------------------------- |
| 1    | `tasks/p5f-a-shell.md`        | Shell, windows, workspaces, project file | 1–9                                |
| 1    | `tasks/p5f-b-inventory.md`    | Glyph inventory, search, filters, data   | 10–18                              |
| 1    | `tasks/p5f-c-editor.md`       | Outline editor tools and panels          | 20–36, 81, 89, 90, 91              |
| 1    | `tasks/p5f-d-metrics.md`      | Metrics, spacing, kerning                | 37–45                              |
| 1    | `tasks/p5f-e-features.md`     | OpenType features and layout             | 46–51, 72, 99, 100                 |
| 1    | `tasks/p5f-f-export.md`       | Font info, design space, export, CLI     | 52–60, 63                          |
| 1    | `tasks/p5f-i-automation.md`   | Cross-document, scripting, plugins       | 77, 78, 79, 96, 105, 106           |
| 1    | `tasks/p5f-n-release.md`      | Packaging and platform acceptance        | 9, 62, release block               |
| 2    | `tasks/p5f-g-bitmap.md`       | Bitmap fonts                             | 64–71, 101–103                     |
| 2    | `tasks/p5f-h-proofing.md`     | Preview, proofing, waterfall, export     | 73–76, 95, 109, 111                |
| 2    | `tasks/p5f-j-construction.md` | Construction, images, optical, virtual   | 82, 83, 85, 86, 87, 92, 93, 94, 97 |
| 2    | `tasks/p5f-k-hinting.md`      | Hinting completion                       | 61, 98                             |
| 2    | `tasks/p5f-l-cjk.md`          | CJK completion                           | 19, 110, 111 (edit view), 112      |
| 2    | `tasks/p5f-m-help.md`         | Help, keymaps, theming, accessibility    | 80, 107, 108                       |
| 3    | follow-ups from reviews       | Remaining Partial rows 9 and 62          | native macOS/Windows, AppImage     |
| 3    | final acceptance              | Row-by-row gate, docs, estimate, release | all                                |

Rows 84, 88 and 104 are already Complete and keep their records.

## Wave 1: done, 2026-09-30

All eight area tasks were implemented, reviewed ("accept with fixes" in every
case) and fix-rounded; a two-pass zh-CN translation followed; the integration
pass then verified the whole tree, fixed the regressions it found, updated the
records and committed `a4b701d` (code) followed by the `docs(p5)` commit
that records the acceptance and status. Result: the audit moves from 3 Complete /
3 Partial / 106 Pending to **75 Complete / 5 Partial / 32 Pending verification**,
with 72 of the wave's 74 rows accepted and rows 9 and 62 left Partial for want of
native macOS/Windows execution and AppImage payload licensing. Integrated
verification is listed in [STATUS](../../STATUS.md); per-row evidence is in
`docs/p5f-{a,b,c,d,e,f,i,n}-*-acceptance.md`.

## Wave 2: done, 2026-10-01

All six area tasks were implemented, reviewed ("accept with fixes" in every
case) and fix-rounded; a two-pass zh-CN translation followed
([1](../../p5f-localization-wave2-pass1.md),
[2](../../p5f-localization-wave2-pass2.md)). The integration pass then verified
the whole tree (`cargo fmt --all --check`, Clippy over the workspace,
`cargo test --workspace --no-fail-fast` 2,086 passed / 0 failed / 2 ignored,
`tsc`, ESLint, Prettier, Vitest 2,774 tests in 339 files, the production build,
and Playwright per configuration), fixed the two cross-task test regressions it
found, updated the records and committed `9b72af8` (code) followed by
the `docs(p5)` commit that records the acceptance and status. Result: the audit
moves from 75 Complete / 5 Partial / 32 Pending verification to **110 Complete /
2 Partial / 0 Pending verification**, with all 35 of the wave's rows accepted.
Integrated verification is listed in [STATUS](../../STATUS.md); per-row evidence
is in `docs/p5f-{g,h,j,k,l,m}-*-acceptance.md` and the wave is summarised in
[the wave 2 checkpoint](../../p5f-wave2-checkpoint.md).

**Wave 3 is not started.** What it covers — rows 9 and 62, the AppImage copyleft
payload licensing, a release rebuild from this code, native macOS and Windows
execution, the English wording items from the translation reports and the final
row-by-row gate — is recorded in [TODO](../../TODO.md). Nothing in this plan
authorises beginning it without a new explicit instruction.

## Per-area cycle

1. Orchestrator writes the task file (scope, rows, ownership, ports).
2. Implementation agent: gap analysis, implementation, tests, browser and
   independent verification, acceptance report, final report.
3. Review agent checks the report against the code and reruns the focused
   checks; defects go back to the implementation agent.
4. After the wave: translation agent (zh-CN), translation review agent,
   integration agent (full verification, fixes, audit table update, commits).

## Exit criteria

- Every Must-have/Should-have row is Complete with linked acceptance evidence,
  or Partial with an explicit, user-visible, documented limit that the parent
  has reviewed and accepted as research-grade.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `pnpm check`, `pnpm lint`, `prettier --check .`,
  `pnpm test`, `pnpm build` and `pnpm e2e` pass in one final run.
- Linux deb and AppImage built, inspected and smoke-tested locally; macOS and
  Windows workflows reviewed and recorded as pending platform execution.
- `docs/STATUS.md`, `docs/TODO.md`, `docs/progress-estimate.md`, user docs and
  the audit table are current; the build cache is within budget.
