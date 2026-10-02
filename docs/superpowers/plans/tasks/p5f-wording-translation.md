# P5f — English wording and independent Chinese delta

Date: 2026-10-02. Status: completed; see
[acceptance](../../../p5f-wording-translation-acceptance.md). Scope: the wording follow-up in `docs/TODO.md` only.

Read `CONVENTIONS.md` and the two `docs/p5f-localization-wave2-pass*.md` reports.
The user explicitly assigns Chinese translation to a GPT-6.1 Sol sub-agent.

## Ownership

- Coordinator: `app/src/i18n/en.json`, directly affected existing test
  expectations, `docs/STATUS.md`, `docs/TODO.md`, final acceptance record.
- Translator: only `app/src/i18n/zh-CN.json` and
  `docs/p5f-wording-translation-pass.md`.
- Preserve every pre-existing uncommitted change. Do not commit, publish,
  modify other product features, or start any remaining backlog task.

## Work

Unify waterfall size terminology while distinguishing a range increment;
match bitmap filter and workspace command names; use curly apostrophes in
English prose; remove the unused bare `cid.unmapped` key while retaining its
counted plural forms. The English delta is available in the working-tree diff
and temporarily in `.scratch/p5f-wording-translation/english-delta.json`.

The independent translator reviews every changed English value in context,
updates Chinese only where needed, removes the same obsolete key, and checks
catalogue keys, interpolation and plural forms. Keep established terminology;
do not rewrite already accurate translations merely to create a delta. Confirm
quoted Chinese control names match their actual labels. Report ambiguity to
the coordinator without silently editing English.

## Acceptance

Run existing localization/spelling tests and catalogue formatting checks.
The coordinator runs `pnpm check`, `pnpm lint`, and `pnpm test` once after the
translation lands. No new tests are needed for this copy-only change.

Run `python3 scripts/check_artifacts.py` at the required resource checkpoints.
Any temporary files belong under `.scratch/p5f-wording-translation/`; use an
absolute task-local `TMPDIR`. The coordinator owns final cleanup.

The translation report stays under 300 words and records changed/removed keys,
terminology decisions, exact check results and verification limits. Do not
claim a new whole-catalogue linguistic or native-platform review.
