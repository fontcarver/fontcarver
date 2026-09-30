# P5 resumed independent localization review

## Scope and plan

Translate the resumed P5 English UI batch independently of feature authors.
Own only `app/src/i18n/zh-CN.json` and this report. Audit key coverage,
interpolation formatting, changed English meanings and existing terminology;
run focused catalogue tests and retain an honest visual-review boundary.

## Result

Completed at the integration owner's English-source barrier on 2026-09-30.
Both catalogues contain **3,120 string leaves**, including 1,279 added Chinese
translations. Seven independent translation reviewers provided 726 proposals;
the localization owner reviewed and merged them against saved English snapshots.

Coverage includes construction, outline/annotation editing, glyph inventory and
smart filters, variable/CJK workflows, proofing, bitmap/image workflows,
cross-document transfers, export, scripting/plugins, workspaces and shortcuts.

Corrected existing master terminology (`主版` → `母版`), an untranslated hinting
label and a stray space. Reviewed semantic changes to FCFP folders, node/handle
snapping, source-preview limits and image tracing that replaces both contours
and components. Final command and kerning-sample additions are included.

## Verification

```text
pnpm --dir app test src/i18n/i18n.test.ts
Test Files  1 passed (1)
Tests       2 passed (2)
Duration    846ms

pnpm --dir app exec prettier --check src/i18n/zh-CN.json
All matched files use Prettier code style!
```

An exact-token audit found zero missing/extra keys, interpolation-format
mismatches or English wording changes pending review. Initial missing keys were
observed failing the existing tests before translation.

Broad frontend checks remain with the integration owner. This pass did not run
browser visual acceptance; catalogue completeness does not establish UI layout
quality. No English/source/API changes or commits were made.

## Resources

No Cargo builds or persistent processes were started. Task scratch was 316 KiB
and was removed after integration. Final `python3 scripts/check_artifacts.py`:
PASS; builds 6.29 GiB / 13,249 files; temporary 0.23 GiB / 7,335 files;
workspace free 668.58 GiB. Checkout inspection shows only the intended Chinese
catalogue modification and new review report in this task's ownership.
