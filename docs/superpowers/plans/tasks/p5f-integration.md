# p5f-integration: close wave 1

Owner: the integration agent (runs alone; no other agent edits the checkout
while this task runs). Protocol: `p5f-protocol.md`, with these differences:
you may run workspace-wide cargo commands and `cargo fmt --all`, you commit,
and you own every file for the purpose of making the tree green. You still do
not push, add features, or translate (a translation agent handles zh-CN).

## Inputs

- `docs/p5f-wave1-checkpoint.md` (state and to-do list), the eight acceptance
  documents `docs/p5f-{a,b,c,d,e,f,i,n}-*-acceptance.md`, and each task file's
  appended gap analysis under `docs/superpowers/plans/tasks/p5f-*.md`.
- Review outcomes (the reviewers reported to the orchestrator, not to files):

| Task | Review verdict    | Fix round result                                                                                       |
| ---- | ----------------- | ------------------------------------------------------------------------------------------------------ |
| n    | accept with fixes | all 9 items fixed (release tests 37/37)                                                                |
| a    | accept with fixes | all 11 items fixed; rows 1–8 Complete on Linux, row 9 Partial (native macOS/Windows pending)           |
| d    | accept with fixes | all 7 items fixed plus the zero-exception regression; rows 37–45 Complete                              |
| i    | accept with fixes | all 9 items fixed (read-only enforcement, transaction reaping); rows 77, 78, 79, 96, 105, 106 Complete |
| e    | accept with fixes | all 8 items fixed; rows 46–51, 72, 99, 100 Complete (row 46 `include` limit stated)                    |
| b    | accept with fixes | all 8 items plus the sidebar consistency fix; rows 10–18 Complete (row 10 ascending-only limit)        |
| c    | accept with fixes | see the final fix-round note appended below                                                            |
| f    | pending           | see the final review/fix note appended below                                                           |

A row becomes **Complete** in `docs/p5-feature-audit.md` only when its
acceptance document recommends Complete and no blocking review defect for
that row remains; otherwise **Partial** with the exact missing piece. Rows
84, 88, 104 keep their existing records.

## Steps

1. Confirm no agent processes remain (`pgrep -af 'fc-devserver|vite|vitest|playwright'`)
   and `python3 scripts/check_artifacts.py` passes. Remove the stray root file
   `./--full-page` and `app/test-results/`; list `.scratch/` and remove only
   `p5f-*` directories left by finished tasks.
2. Small integration fixes (yours to own):
   - `crates/fc-api/src/bin/gen_ts.rs`: write into a temporary directory next
     to `app/src/api/generated/` and swap atomically, so a running Vite never
     serves an empty directory; keep every export block.
   - `app/playwright.config.ts`: `workers: 1` and `fullyParallel: false`
     (specs share one kernel document and time out with two workers).
   - `app/e2e/p2a-kerning.spec.ts:322`: match the tab with `(?:^|· )Regular\.ufo$`.
   - Anything the appended notes below list.
3. Regenerate and verify, in this order, fixing regressions minimally and
   never by deleting or skipping tests: `pnpm --dir app gen:api`,
   `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace --no-fail-fast`, `pnpm --dir app check`,
   `pnpm --dir app lint`, `pnpm --dir app exec prettier --check .`
   (fix with `--write`; `THIRD_PARTY.md` and `crates/fc-doc/src/fcf/schema.md`
   had pre-existing drift, format them too), `pnpm --dir app exec vitest run --maxWorkers=4 --minWorkers=1`,
   `pnpm --dir app build`, then Playwright for every config
   (`playwright.config.ts`, `e2e/p2a.config.ts`, `e2e/p2b.config.ts`,
   `e2e/p2cd.config.ts`) on ports 8787/1420 with an absolute `TMPDIR` under
   `.scratch/p5f-integration/tmp`. If the zh-CN parity test fails, list the
   missing keys in your report; do not translate them yourself.
4. Update records:
   - `docs/p5-feature-audit.md`: every wave 1 row per the rule above, with a
     link to its acceptance document in the evidence column; update the
     header paragraph and the status counts.
   - `docs/STATUS.md`: replace the wave 1 paragraph with the integrated
     result (test counts, commits), keep the phase table, note that wave 2 is
     planned and not started.
   - `docs/TODO.md`: rewrite the remaining-scope table from the current audit
     (wave 2 areas, Partial rows with their missing pieces, review defects that
     remain, the AppImage licensing gap, native macOS/Windows execution).
   - `docs/progress-estimate.md`: add a dated reassessment section; adjust
     only the completion bands of areas with newly accepted rows, conservatively,
     and recompute the weighted band with the same arithmetic.
   - `docs/superpowers/plans/2026-09-30-p5-completion.md`: mark wave 1 done
     with the commit hashes; `docs/p5f-wave1-checkpoint.md`: final paragraph.
5. Commit in two commits (the wave's changes are interdependent; intermediate
   per-area commits would not build): `feat(p5): integrate completion wave 1`
   (all non-docs paths; summarise per area and state the verification results
   you observed) and `docs(p5): record completion wave 1 acceptance and status`.
   Check staged paths as wave 0 did (no generated types, no files over 2 MB,
   no `target/`, `node_modules/`, `fontforge/`).
6. Finish with per-member `cargo clean -p …` for the 15 workspace members (as
   wave 0 did) so the cache is under 10 GiB, `python3 scripts/check_artifacts.py`,
   and a report (under 500 words): every command with counts, every fix you
   made, the audit status counts before/after, the commit hashes, and anything
   left failing or deferred with the reason.
