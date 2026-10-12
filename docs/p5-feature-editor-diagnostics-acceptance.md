# P5 feature source diagnostics, completions and class preview

Completed the bounded work on 2026-09-30; no commits or source-authority changes.

Located diagnostics now show one-based source line/character columns while
retaining original UTF-16 navigation spans. Tests cover CRLF, lone CR, emoji,
source-level diagnostics and missing source identities. Completion inventory and
dynamic loader identity are tied to document/generation; delayed or failed
refreshes cannot restore old names.

Alt-click or Alt+Enter on a source `@Class` opens a read-only preview. It shows
names and current-master outlines, supports nested classes, comments, escaped
names and spaced alphanumeric ranges, and identifies missing/cyclic/duplicate
definitions. Comments, strings and composition do not trigger keyboard preview.
The parent wired candidate source, master and generation into the source editor.

Preview resolution is bounded: 512 unique members, 32 recursion levels, 4,096
classes, 128 KiB inspected source and 8,192 member operations. Display is capped
at 64 members and searches at eight concurrent requests. Compact hyphenated
names/ranges, predicates and CIDs are explicitly marked incomplete; compiler
validation remains authoritative.

Verification:

- Diagnostics/language/editor/completion Vitest suites: **4 files, 28 tests passed**.
- Class preview/parser suites: **2 files, 20 tests passed**.
- Source-editor keyboard/read-only/outline integration: **1 test passed**.
- Existing FeaturesView regression: **6 tests passed** with one worker. An earlier
  parallel run hit four five-second timeouts during shared CPU contention; the
  isolated rerun completed in 7.63 seconds.
- Scoped ESLint and Prettier checks passed. Tests were written before fixes.
- Parent owns broad frontend checks. Browser acceptance was not completed:
  browser startup failed under resource pressure, and the root stopped retries.
  New English keys are `features.diagnosticLocation` and
  `features.classPreview.*`; independent translation remains with its owner.

Owned scratch and test processes were cleaned. Final artifact checkpoint passed:
build 6.11 GiB / 13,180 files, temporary 0.23 GiB / 7,968 files, 668.69 GiB free.
Work stopped after current tasks as requested.
