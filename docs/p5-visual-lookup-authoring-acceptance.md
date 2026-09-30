# Visual lookup authoring acceptance — 2026-09-30

## Delivered

Features now includes a Visual editor with typed forms for GSUB 1/2/3/4/6 and
GPOS 1/2/4/6. Supported feature bodies parse back into the forms; edits render
canonical FEA and use the existing draft persistence, validation, checked apply,
discard and document history workflow. Incomplete operands remain saved as
invalid FEA drafts; a session cache retains their form state across mode and
entry navigation. Source and visual modes share the selected feature identity.

Imported authority and generated blocks remain protected. Unsupported syntax is
preserved and can be opened in the source editor. New mark classes receive unique
names. The applied shaping preview explicitly compares the selected feature off
and on, showing actual glyph names, advances and offsets; changed context and
stale shaping invalidate the comparison. Pending drafts are clearly identified.

## Actual checks

- `pnpm test src/views/features/visualLookup.test.ts`: 43 passed.
- `pnpm test src/views/features/VisualLookupEditor.test.tsx src/views/features/FeaturesView.test.tsx --maxWorkers=1`: 12 passed.
- After preview/selection integration:
  `pnpm test src/views/features/VisualLookupEditor.test.tsx --maxWorkers=1 --testTimeout=30000`: 6 passed.
- `VisualLookupPreview.test.tsx`: 8 passed; scoped ESLint passed.
- Scoped ESLint for VisualLookupEditor/Fields/tests/FeaturesView: passed.
- Targeted Prettier: passed.
- Backend `cargo test -p fc-api --test visual_lookup_authoring` has 12 acceptance
  tests covering actual nine-type shaping, glyph classes, advances, lookup table
  kinds, checked apply/rejection, undo/redo and FCF persistence. This run failed
  during unrelated in-progress compilation before these tests executed. Root
  integration owns their final execution; they are **not yet passed** here.
- Earlier broad frontend checks encountered concurrent generated-API drift and
  unrelated source errors. Root owns consolidated typecheck/lint/regression.

Agent-browser skill was read and isolated headless verification attempted on
owned ports 8879/1479. Chrome exited before DevTools initialization, including
one diagnosed retry. Root then froze new browser work under resource pressure.
No visual browser acceptance is claimed. Both owned servers and browser session
were stopped, and `.scratch/p5-feature-authoring` was removed.

Final artifact checkpoint: PASS; build 6.11 GiB, temporary 0.22 GiB, workspace
free 668.66 GiB. No source, font, shared cache or other task files were removed.

## Acceptance boundaries and audit guidance

The parser is a bounded typed statement IR, not a complete FEA AST. It supports
one visual rule per feature block (mark definitions plus their attachment rule),
literal glyph lists/classes, integer value records and simple single-input
contextual replacement. Lookup flags, named contextual graphs, language scopes,
scalar positioning with ambiguous axis, class references, device/variation
records and mixed feature bodies stay in the source editor. An invalid draft
reopened in a new session remains editable as source; its in-memory form cache
is intentionally not persisted.

Rows 46 and 99 should record these bounded implementations and remain partial
against a requirement for arbitrary FEA/lookup round-tripping. Row 50 additions
and evidence are in `p5-feature-editor-diagnostics-acceptance.md`; browser
acceptance remains pending. Row 100 syntax, tests and explicit DSL limits are in
`p5-feature-tokens-acceptance.md`. Existing rows 47–49 were reused; no claim of
new full suffix-generation coverage is made by this task. No commits made.

## Integration regression corrections

The root workspace run reached the backend suite: seven tests passed and five
failed on test assumptions. Installed HarfRust `hb/ot/gpos/mod.rs` applies
XAdvance only to horizontal runs and YAdvance only to vertical runs (with the
vertical advance sign convention). Expectations now keep horizontal YAdvance
zero and explicitly check vertical feature-off/on deltas for authored 7/6-unit
adjustments, while retaining both placement checks. HarfRust
`hb/ot_shape.rs` suppresses fallback mark positioning whenever it applies GPOS,
so `mark=0` in a font with GPOS need not equal a font with no GPOS. Disabled runs
now also compare the updated shaping cache against a fresh full compilation of
the same source; non-mark baseline comparisons and feature-effect assertions
remain. Root owns the corrected rerun.

The same integration pass found two feature-token regressions: raw oversized
source was categorized as a token error before the editor's source limit check,
and a model test assumed 1000 UPM while constructing a zero-UPM `Font::default`.
The raw source budget is now checked before expansion and the fixture explicitly
uses `Font::new_empty(..., 1000)`. No token/shaping production semantics were
relaxed. Targeted formatting passed; corrected test execution is pending root's
serialized check at this checkpoint.
