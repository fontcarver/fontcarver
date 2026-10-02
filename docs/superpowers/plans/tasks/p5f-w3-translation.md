# Independent GPT-6.1 Sol Chinese translation delta

User explicitly assigned Chinese translation to GPT-6.1 Sol. Reuse that
independent translation agent. Read CONVENTIONS.md. Own only
app/src/i18n/zh-CN.json, this task and docs/p5f-w3-translation-acceptance.md.

Compare en.json with committed a4be04a to identify all wave 3 changes. Spacing
keys and bitmap save/vertical proofing keys are ready; Fix All wording will
arrive from parent/owner shortly. Translate new values naturally, preserve
placeholders/plurals, verify actual UI terminology, and independently review
changed existing values. Never edit English or feature code. Coordinate final
key freeze before claiming parity; parent will run full integration checks.

Run focused i18n/spelling tests and catalogue format/parity/placeholder checks
with own scratch/TMPDIR and artifact checks. Do not spawn agents or commit.
Report reviewed key list, actual tests, remaining limitations; preserve report
and remove owned scratch/processes.
