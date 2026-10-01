# P5 completion protocol (shared by every `p5f-*` task)

This file is referenced by every task in the P5 completion waves
(`docs/superpowers/plans/2026-09-30-p5-completion.md`). Follow it exactly.

## Read first

`AGENTS.md`, `CONVENTIONS.md`, `docs/STATUS.md`, `docs/TODO.md`, your task
file, your rows in `docs/p5-feature-audit.md`, your rows and the bounded
acceptance checklist in `docs/p5-resume-audit.md`, and every acceptance or
checkpoint document your task file names. Then read the existing code for your
area before changing it: the 2026-09-30 batch already implemented a large part
of most rows, and the gap between "implemented" and "accepted" is your work.

## Working mode

- **Shared checkout, many agents at once.** Edit only the files in your
  ownership list, plus narrow patches to the shared files your task allows.
  Re-read a shared file immediately before editing it, change only the lines you
  need, never reformat it, and keep JSON files valid. Never run `git stash`,
  `git checkout -- <path>`, `git reset`, `git clean`, `git commit` or `git push`.
  Never revert, rewrite or reformat another task's files. If a file you need is
  owned by another task, implement what you can, and name the exact file and
  change you need in your report.
- **No subagents.** Do the work yourself.
- **No native or global desktop input automation** (no xdotool/ydotool, no real
  desktop). Headless browsers only.
- **Scratch and budgets.** Use `.scratch/p5f-<area>/` for everything temporary
  and export an absolute `TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5f-<area>/tmp`
  for every test command. Run `python3 scripts/check_artifacts.py` at start,
  between build batches and before your report. Before reporting: stop every
  process you started, keep only evidence cited from `docs/`, delete your scratch
  directory.
- **Cargo.** Wrap every cargo command:
  `mkdir -p .scratch/p5f && flock .scratch/p5f/cargo.lock <cargo command>`.
  Use focused runs (`cargo test -p <crate> [--test <name>]`,
  `cargo clippy -p <crate> --all-targets -- -D warnings`); never `--workspace`.
  Batch your edits so you run cargo a few times, not after every line. Format
  only your own files with `rustfmt --edition 2021 <files>`, never
  `cargo fmt --all`. If a crate you did not touch fails to compile, another agent
  is mid-edit: wait two to three minutes and retry, up to three times, then
  report it and continue with independent work.
- **Frontend.** `pnpm --dir app exec vitest run --maxWorkers=1 --minWorkers=1 <your test files>`,
  `pnpm --dir app check`, `pnpm --dir app exec eslint <your dirs>`,
  `pnpm --dir app exec prettier --write <your files>`. The full Vitest suite,
  `prettier --check .`, the production build and Playwright belong to the
  integrator. You may add Playwright specs under `app/e2e/` for your rows and run
  them with `FC_API_PORT=<API> FC_UI_PORT=<UI> pnpm --dir app exec playwright test <spec>`.
- **i18n.** Every user-visible string goes through `t()` with keys in
  `app/src/i18n/en.json` (append under your namespace with small edits). Never
  edit `app/src/i18n/zh-CN.json`; a separate translation pass runs after the
  wave, so `src/i18n/i18n.test.ts` is expected to fail until then. List every
  new key namespace in your report.
- **Generated API types.** Never hand-edit `app/src/api/generated/`. After
  changing `fc-api` types run `flock .scratch/p5f/cargo.lock pnpm --dir app gen:api`
  (it compiles the workspace; on another agent's compile error, retry later).
- **Ports.** Your task assigns an API port and a UI port. Browser recipe:
  1. `flock .scratch/p5f/cargo.lock cargo build -p fc-devserver`
  2. `target/debug/fc-devserver --port <API> [--open <font path>] &`
  3. `cd app && FC_DEV_PROXY=http://127.0.0.1:<API> VITE_FC_API=http://127.0.0.1:<UI> pnpm exec vite --port <UI> --strictPort &`
  4. Drive `http://127.0.0.1:<UI>` with the `agent-browser` skill (invoke it
     through the Skill tool first). Save only the screenshots your acceptance
     report cites under `docs/screenshots/p5f-<area>/` (PNG, under 300 KB each).
  5. Kill both processes when done.
- **Dependencies and licensing.** A new crate or npm package needs a permissive
  license (CONVENTIONS.md), a `THIRD_PARTY.md` entry and a one-line justification
  in your report. FontForge code may be adapted only from files whose own header
  is 3-clause BSD, with the attribution comment and `THIRD_PARTY.md` row; name
  every FontForge file you drew from and its header license in the report.

## Method

1. **Gap analysis first.** For each audit row you own, compare the requirement
   text with the code as it exists now (read it, run it in the browser). Append a
   `## Gap analysis` section to your task file: per row, what works, what is
   missing, what you will implement, and what you will record as a limit and why.
2. **Implement gaps test-first** (failing test, implementation, refactor). Aim
   for the full professional behaviour the row describes. A documented limit is
   acceptable only for a genuinely research-grade sub-feature, never as a
   substitute for the ordinary workflow. No placeholder controls, no silently
   dropped data, no "coming soon".
3. **Every editing feature needs**: a user entry point (menu, command palette,
   panel or shortcut), deterministic behaviour, invalid-input rejection that
   leaves the document intact, one undo/redo unit per gesture, `.fcf`
   save/reopen, export where relevant, keyboard reachability and `aria-label`s.
4. **Verify against the real kernel** in the headless browser for each row's
   main workflow, and check emitted fonts independently with FontTools,
   FreeType and HarfBuzz (`python3` has `fontTools`, `freetype`, `uharfbuzz`).
5. **Keep the docs true.** Update `crates/fc-doc/src/fcf/schema.md` with every
   model change and `docs/user/` (English) for every new user-facing workflow.
6. **Write `docs/p5f-<area>-acceptance.md`**: scope, exact commands with their
   summary output, browser evidence, remaining limits, and a per-row recommended
   status. Do not edit `docs/p5-feature-audit.md`, `docs/STATUS.md`,
   `docs/TODO.md` or `docs/progress-estimate.md`; the parent owns them.

## Final report to the parent (under 500 words)

- Per row: **Complete** / **Partial** (the exact missing piece) / **Not done** (why).
- Commands run with pass/fail counts (focused tests, clippy, tsc, eslint).
- New i18n key namespaces; shared files touched; new dependencies.
- Anything blocked and what you need from whom.
- Final `check_artifacts.py` output and confirmation that your processes and
  scratch are gone.

## Lessons from wave 1 (binding for wave 2)

- Never run `agent-browser close --all`; name your session (`p5f-<area>`) and
  close only it. Headless Chrome in this checkout needs the shell sandbox
  disabled for agent-browser and Playwright runs.
- `pnpm gen:api` now stages atomically, but still run it under the cargo flock.
- The default Playwright config runs with one worker and autosave disabled;
  e2e fixtures must close only the documents they opened. Use your own ports.
- Give every row's acceptance a kernel-level check of invalid input and locks:
  wave 1 reviews found data loss (encodings overwritten), lock bypasses
  (`AlignComponents`) and read-only bypasses that unit tests had missed.
- Shared files touched by several wave 1 tasks (`crates/fc-api/src/api.rs`,
  `bin/gen_ts.rs`, `handlers/mod.rs`, `types/mod.rs`, `fc-doc/src/command/apply.rs`,
  `fcf/schema.md`, `app/src/shell/builtinCommands.ts`, `app/src/i18n/en.json`):
  re-read immediately before every edit; additive hunks only.
- Finish with a "Review round" section in your acceptance document when a
  reviewer's defects come back; state per item what changed and which test
  proves it.
