# P5-43 kerning table and maintenance

Scope: a reachable Text view kerning table for horizontal and vertical records,
with generation-consistent pagination, sorting, filtering, pair removal,
class-aware maintenance, table copy/paste across masters, and group rename with
an explicit all-master scope confirmation. Existing source ownership guards,
document undo and edit barriers remain authoritative.

Owned frontend files: `app/src/views/text/KerningTable*`,
`KerningRenameDialog.tsx`, `kerningTableData*`, and the `kerningTable` English
translation namespace. TextView integration is coordinated with the proofing
and kerning-mode owners. Backend maintenance is delegated to the maintenance
agent, which owns its own bounded task plan and reports the generated command
contract. Separate delegates own class defaults, mode protection and ghosting.

Acceptance: table pages retain a single document generation; cancellation and
stale replies cannot publish obsolete state; sorting/filtering precede visual
pagination; copying preserves class records and explicit zero; paste targets the
chosen current master/direction and carries the current generation; mutations
wait for pending edits and register a save barrier; rename states its scope
before applying. API rejects stale or invalid maintenance atomically and all
mutations participate in undo.

Verification: focused Vitest data/component cases and focused backend semantic
tests; artifact checks at start/end. Final broad TypeScript, lint, Rust and
browser acceptance is integrated by the parent under the resume coordination.

## Frontend result

Implemented the collapsed Text view table and coordinated its insertion with
the proofing owner. All source records are read through bounded, cancellable,
generation-consistent API pages. Filtering and sorting cover the whole table;
the visible table renders 50 rows per page in a bounded scrolling region.
Class records, explicit zero, and invalid imported diagnostics remain visible.
Copy retains the complete valid source table across master selection; paste
merges or explicitly replaces the current master's current direction. Bulk
commands carry the generation observed by the table. The group rename dialog
confirms that both directions and every master are affected.

Mutations use the existing kerning edit barrier, so saves and controller
handover wait for them. Errors are reported and the source table is refreshed.
Imported/source-owned layouts remain read-only. No dependencies or FontForge
source were used. New English keys are in `kerningTable`; the class shortcut
hint was added to `kerningText` for the independent class-edit task.

Focused verification:

```text
pnpm exec vitest run --maxWorkers=1 src/views/text/KerningTable.test.tsx src/views/text/kerningTableData.test.ts
Test Files  2 passed (2)
Tests       11 passed (11)

pnpm exec eslint <seven owned/touched TypeScript files>
exit 0
```

Covered lazy loading, single-generation continuation pages, cancellation,
generation mismatch rejection, class-name filtering, numeric sort/zero,
cross-master clipboard data, direction-specific clipboard guards, read-only
imported layouts, mutation protection when kerning mode is off, vertical maintenance targeting and explicit all-master rename
confirmation. An initial test-only unused destructured field
was corrected before the successful scoped lint run.

The backend maintenance task supplies atomic validation, class-aware compression,
undo/redo and FCF persistence tests. Final generated API, broad frontend checks,
and integrated browser acceptance remain with the integration owner. Row 43
must not be marked fully verified before those checks complete. Work stopped at
this scope after the user asked to finish tasks already underway and rest.

Final resource checkpoint: PASS, 6.11 GiB build, 0.23 GiB shared temporary data,
668.64 GiB free. This task's empty scratch directory was removed and no owned
process remains. Final owned-file Prettier and ESLint checks passed.
