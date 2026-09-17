# P2b — Editable OpenType layout acceptance

Date: 2026-09-17.
Status: tasks 01–11 complete within the documented static conversion scope.
Implementation checkpoint: `80d72e1` (`feat(features): complete P2b editable OpenType layout`).
Scope: [design](superpowers/specs/2026-09-17-p2b-editable-layout.md) and
[tasks 01–11](superpowers/plans/2026-09-17-p2b-editable-layout.md).

## Delivered behavior

- A Features tab with CodeMirror FEA editing, source tree, glyph/class completion,
  located UTF-16 diagnostics, all-master validation and explicit application.
  Invalid drafts survive native save/recovery without replacing active layout.
- Document-owned undo and coordinated save, export, tab, master, history and
  close handovers. Tests cover input arriving during persistence, IME composition,
  stale replies, recovery, failed saves and existing outline/spacing controls.
- Explicit automatic recipes for small capitals, stylistic sets, case/vertical
  forms and unambiguous ligatures; explicit script/language mappings for `locl`
  and `dlig`. Generated source can be frozen, with writer ownership preventing
  duplicate kerning/mark execution.
- Bounded imported-table/lookup/rule inspection with rendered glyphs and paged
  references. Unsupported registrations retain useful read-only rule inspection.
- Complete static source candidates for GSUB 1–8, GPOS 1–9 and representable
  GDEF/BASE. Every conversion requires actual compilation for every master,
  canonical graph equivalence and supplementary shaping checks. Tokens bind the
  exact server-held candidate to the document's current revisions and provenance.
- Proven horizontal pair/group and bounded mark/mkmk anchor transfer, including
  combined transfers. Failed optional transfers retain source-owned rules.
  Native reopen, original-layout restore and undo preserve unrelated edits.
- All external exports reject unapplied drafts. Original-table fonts require
  validated conversion before UFO/designspace export, including direct writers
  and the CLI. Source export revalidates active FEA. UFO writer policy survives
  roundtrip; flattened generated source reopens frozen. Only native documents
  retain the original-layout editing backup.
- An independent Chinese translation pass added 144 keys. English and Chinese
  each contain 1,115 keys with matching placeholders.

## Automated verification

| Gate | Result |
| --- | --- |
| `fc-model`, `fc-doc`, `fc-formats`, `fc-compile`, `fc-layout`, `fc-api` combined suites | 644 passed; 2 existing release-only tests ignored; 1 paused Source Han timing test filtered |
| `fc-sfnt` TTF, OTF, containers and malformed-input suites | 17 passed |
| CLI integration suite | 12 passed |
| Frontend Vitest suite | 126 files, 1,075 passed |
| P2b real-backend Playwright | 10 passed in 33.7 seconds after rebuilding the final Rust backend |
| Workspace all-target Clippy and Rust formatting | Passed |
| Generated TypeScript API, TypeScript, ESLint and production build | Passed |

The combined Rust gate excludes
`source_han_export_meets_release_budget_when_available`. Existing ignored gates
are `edits_three_dejavu_glyphs_and_preserves_layout_on_export` and
`edit_apply_meets_budget_at_twenty_thousand_glyphs`; no release-performance
claim is inferred from this run. Later bounded conversion-report unit tests and
11 final conversion-disclosure UI tests also passed. Production build retains
Vite's size advisory: the main JavaScript
chunk is about 1.09 MB minified / 348 KB gzip.

Local command evidence lives in `target/p2b-rust-acceptance.log`,
`target/p2b-sfnt-acceptance.log`, `target/p2b-cli-tests.log`,
`target/p2b-frontend-tests.log`, `target/p2b-clippy-workspace.log`,
`target/p2b-rustfmt.log`, `target/p2b-typescript-check.log`,
`target/p2b-eslint.log` and `target/p2b-production-build.log`.
Logs and generated API files are intentionally not committed.

## End-to-end and visual checks

The [isolated Playwright configuration](../app/e2e/p2b.config.ts) uses dedicated
ports, temporary fonts/configuration and its own browser state. Its tests cover
invalid Unicode draft save/reopen, validation versus activation, generation and
freeze, stale responses, save failures, IME, tab/master transitions, imported
glyph rendering, conversion, binary/source exports, source-only fallback,
kerning edits and original-layout restore after native reopen.

Interactive `agent-browser` checks use a separate server/profile from Playwright.
The [English/light source](screenshots/p2b-features-en-light.png),
[Chinese/dark source](screenshots/p2b-features-zh-dark.png) and
[Chinese/dark lookup](screenshots/p2b-lookups-zh-dark.png) screenshots were
independently inspected for readable controls, diagnostics, source, generation
settings and real glyph outlines. A final visual polish keeps optional compiler
proof details collapsed; actual blockers remain visible. The four imported
Playwright tests passed again after that change, followed by all ten workflows
against a freshly rebuilt final backend. Owned browser sessions, servers and
temporary visual fixtures were closed/removed after verification.
See [capture details](screenshots/p2b-captions.md).

## Independent review

Kernel and frontend reviewers were separate from their implementation authors;
Chinese translation was a separate pass. Resolved findings include:

- preserving distinct lookup identities and shared execution across features;
- rejecting empty language fallbacks, unsorted records and shared-offset memory
  amplification rather than normalizing away meaningful data;
- verifying the shaping baseline's actual original table bytes;
- preserving generated rules in the presence of parser warnings;
- guarding revision exhaustion, complete kerning-state consistency and disabled
  model-writer editing;
- preventing held-save A → B → A data loss, late IME persistence, stale clean
  close decisions, misleading post-apply diagnostics and freezing an older
  generated snapshot than the one displayed.

Final independent scoped evidence includes 48 import tests, 10 lifting tests,
9 conversion API tests, 59 Features tests and 39 handover/navigation tests.
All reported review blockers were resolved.

## Supported scope and remaining boundaries

The executable [capability matrix](../crates/fc-layout/tests/LAYOUT_CAPABILITIES.md)
lists supported formats, compiler restrictions, negative fixtures and actual
licensed-corpus outcomes. Static lookup-number coverage does not mean every
font using those lookup numbers can be converted. Legacy `kern`/AAT competitors,
JSTF, variable layout, unsupported FeatureParams/device/point data, resource
limits and unproven graph transformations block the complete conversion.
UFO writer metadata supports the current model's kerning and combined anchor
writer policies in Skip mode. Append mode, custom writers or independent
mark/cursive/GDEF subsets produce explicit import errors rather than silently
approximating their behavior.

The original licensed DejaVu, Source Sans/Serif and Noto fixtures retain explicit
blockers. Tibetan source compiles but complete GPOS equivalence remains
unproven; it stays in original-table mode. Positive conversion evidence comes
from compiler-built fixtures covering the accepted structures and model-writer
spacing/anchor roundtrips. There is no partial or “convert anyway” path.

P2c CFF export, P2d format work, native desktop automation, the paused Source Han
timing investigation, FontForge comparison and release packaging are outside
this acceptance.
