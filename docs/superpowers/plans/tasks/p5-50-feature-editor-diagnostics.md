# P5-50 Feature editor diagnostic locations and completion freshness

## Scope

Finish concrete source-editor gaps in feature audit row 50: show the source
line and column for located diagnostics, keep navigation anchored to the source
identity and UTF-16 span, and prevent glyph completion names from leaking across
document or generation changes. Expanded by the feature-authoring parent to
include Alt-click and keyboard class preview directly from source references.

## Owned files and interfaces

- `app/src/views/features/FeatureDiagnostics.tsx` and focused component tests.
- `app/src/views/features/featureLanguage.ts` and its offset/location tests.
- `app/src/views/features/useFeatureCompletions.ts` and focused hook tests
  (delegated to `completion_refresh`).
- The `features.diagnosticLocation` English translation key.
- `FeatureCodeEditor.tsx` and its interaction tests; a class-reference callback.
- `FeatureSourceEditor.tsx` narrow preview integration, with optional master and
  generation props wired by the parent in `FeaturesView.tsx`.
- New `FeatureClassPreview` component/styles/tests and `featureClassMembers`
  helper/tests, delegated to `class_preview`; `features.classPreview.*` keys.

No change to the kernel API, document persistence, `FeaturesView.tsx`, or lookup
authoring. Diagnostic offsets stay UTF-16 source offsets; display locations use
one-based lines and Unicode character columns with CRLF treated as one newline.
Completion inventory is scoped to both document and generation; stale async
responses must not reappear.

## Acceptance

Write failing tests before implementation. Verify localized line/column labels,
UTF-16 navigation spans across Unicode and mixed newline forms, source-level
diagnostics without fake locations, absent sources, and refreshed completion
inventory after document/generation changes and failed/late queries. Run focused
Vitest coverage, scoped lint/format verification, and artifact checks. Parent
owns broad integrated frontend checks and browser acceptance. Record actual
results and boundaries in a scoped acceptance report; no commits or shared
status/audit edits.

Class preview must open for Alt-click and Alt+Enter on a class reference, show
actual member names and available glyph outlines, and remain non-editing and
keyboard accessible. Recursive resolution and outline requests are bounded;
unsupported/unresolved source must be visibly identified. Comments and strings
must not trigger preview, and changing source/doc/master invalidates stale
preview data.
