# P2a-04 — Chinese localization and independent QA

## Scope

Translate the completed P2a horizontal kerning interface independently from its
feature authors, as required by `CONVENTIONS.md`. Preserve existing translations
and unrelated workspace changes. Review the interface wording for the P2a
ownership restrictions, pair scopes, explicit zero values, inherited values,
group impact, and ambiguous text gaps.

## Owned files

- `app/src/i18n/zh-CN.json`
- `docs/p2a-acceptance.md` (section “Localization QA”)
- This task file.

English catalogues and frontend behavior remain owned by the feature authors.
Report missing English strings or translation mappings to the coordinating
agent instead of modifying those files.

## Public interfaces

The feature authors supply English objects for the top-level `kerningGroups`,
`kerningText`, and `kerningCanvas` catalogue sections. The coordinating agent
merges these into `app/src/i18n/en.json`. Translate any additional agreed English
error/warning keys and changes to existing labels in the same pass.

Use the established vocabulary for glyphs (`字形`), masters (`母版`), and
sidebearings (`侧边距`). Use `字偶距` for kerning and `字偶距组` for kerning groups.
Group sides refer to pair order, so label them `第一侧` and `第二侧`.

## Acceptance

1. English and Simplified Chinese leaf keys match exactly.
2. Interpolation placeholders remain identical, including number-format hints.
3. New strings distinguish an explicit zero from removing a record to inherit.
4. Unsupported layout and ambiguous-gap messages use product language.
5. New visible labels and accessibility strings are routed through translation.
6. Record `pnpm check`, `pnpm lint`, and `pnpm test` results, separating unrelated
   concurrent failures from localization issues. Run the focused catalogue tests
   after the final merge.
7. Report the reviewed scope and any visual or functional checks performed by
   another agent without claiming they were independently executed here.

## Status

- [x] Read conventions, P2a design, and existing translation terminology.
- [x] Send terminology and missing-message findings to feature authors.
- [x] Translate final English keys.
- [x] Validate key/placeholder parity and review feature-code label coverage.
- [x] Record final QA results in `docs/p2a-acceptance.md` (section “Localization QA”).
