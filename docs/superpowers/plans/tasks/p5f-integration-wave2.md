# p5f-integration-wave2: close wave 2

Owner: the integration agent (runs alone; no other agent edits the checkout
while this task runs). Protocol: `p5f-protocol.md`, with the same differences
as `p5f-integration.md`: workspace-wide cargo commands and `cargo fmt --all`
are allowed, you commit, you own every file for the purpose of making the
tree green. You do not push, add features, or translate.

## Inputs

- `docs/STATUS.md`, `docs/TODO.md`, `docs/p5-feature-audit.md` as left by the
  wave 1 integration (`976d213`), the six acceptance documents
  `docs/p5f-{g,h,j,k,l,m}-*-acceptance.md`, and each task file's appended gap
  analysis under `docs/superpowers/plans/tasks/p5f-*.md`.
- Review outcomes (filled in by the orchestrator before this task starts):

| Task | Review verdict    | Fix round result                                                                                                                                                 |
| ---- | ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| g    | accept with fixes | 7 items (SetStrike lock scope, unencoded single-glyph waterfall, retained script, preset docs, tEXt decoding); fix round pending at the time of writing          |
| h    | accept with fixes | 5 items (range example vs step cap, TSV `#`/trailing-space loss, keyed rejection messages, tri-state test, stale hint); fix round pending at the time of writing |
| j    |                   |                                                                                                                                                                  |
| k    |                   |                                                                                                                                                                  |
| l    | accept with fixes | all 5 items fixed plus a VerticalMetricsPanel stale-override defect found by new e2e; rows 19, 110, 111 (Edit View/VORG), 112 Complete with stated limits        |
| m    | accept with fixes | all 5 items fixed (quick-help button reachable by pointer and keyboard, spelling, a11y guards, `--bm-*` test); rows 80, 107, 108 Complete                        |

A row becomes **Complete** in `docs/p5-feature-audit.md` only when its
acceptance document recommends Complete and no blocking review defect for
that row remains; otherwise **Partial** with the exact missing piece.

## Steps

Follow `p5f-integration.md` steps 1–6 with these wave 2 specifics:

1. Step 1 as before (no agent processes, stray files, `.scratch/p5f-*`).
2. Integration fixes: whatever the appended notes below list. Do not touch
   `app/src/i18n/en.json` or `zh-CN.json`; the translation passes have run
   after task m's English edits. If the parity test fails, list the keys.
3. Verification order and commands as in wave 1 (gen:api, fmt, clippy
   workspace, `cargo test --workspace --no-fail-fast`, tsc, eslint, prettier,
   vitest, build, Playwright for all four configs with one worker). Fix
   regressions minimally, never by deleting or skipping tests.
4. Records: audit rows for 61, 64–76, 80, 82, 83, 85–87, 92–95, 97, 98,
   101–103, 107–112, 19 (and any wave 1 Partial row the acceptance documents
   now close); `STATUS.md` (wave 2 integrated; what remains for wave 3);
   `TODO.md` (wave 3 scope: Partial rows, review leftovers, AppImage licensing,
   native macOS/Windows runs, final gate); `progress-estimate.md` dated
   reassessment with the same arithmetic; plan file wave table; a short
   `docs/p5f-wave2-checkpoint.md` summarising the wave like the wave 1 one.
5. Two commits: `feat(p5): integrate completion wave 2` and
   `docs(p5): record completion wave 2 acceptance and status`, with the same
   staging checks as before.
6. Per-member `cargo clean -p …`, `python3 scripts/check_artifacts.py`, and the
   report (under 500 words) with every command's counts, every fix, audit
   counts before/after, commit hashes, and anything left failing or deferred.

## Appended notes

- Task m's English wording edit `kerningTable.classRule` ("Class" → "Group")
  breaks task d's `app/src/views/text/KerningTable.test.tsx` expectation; update
  the test string (narrow patch).
- Task h fixed the `text.spec.ts:122` regression: it was the cold first
  `text.shape` compile (about 4 s), not dropped input; the spec now waits on
  `data-text-shaping`. Correct `docs/TODO.md` accordingly.
