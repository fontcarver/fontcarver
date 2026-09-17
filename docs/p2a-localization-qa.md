# P2a localization QA

Date: 2026-09-16.
Result: Chinese catalogue and static interface wording review pass.

The translation agent did not implement the feature or write its English copy.
Scope followed [P2a-04](superpowers/plans/tasks/p2a-04-localization.md).

## Reviewed content

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

## Verification

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

## Review fixes

Feature authors addressed untranslated local errors by using mapped API error
codes. The group creation path now rejects duplicate group names atomically,
after review found that the previous replacement command could clear members.
Transport-failure copy now asks the user to check stored membership instead of
claiming that no change occurred. Existing unrelated Chinese edits were retained.
