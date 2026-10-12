# P5 project proof library data acceptance

Built bounded, versioned project proof metadata at
`FontInfo.user_data["com.fontcarver.proofLibrary"]`, exact Unicode/glyph-escape
preservation, group/filter/cycle helpers and a React persistence hook. Saves and
removals read current font metadata inside the existing settled-document barrier
and apply one `SetFontInfo` command, preserving unrelated metadata and command
undo semantics. Malformed, future-version and incompatible metadata refuse
writes; failures retain visible saved entries. Late document loads/saves cannot
replace another project's library.

Validation was test-first: both new test files initially failed because their
implementation imports did not exist. A malformed array direction regression
also failed before strict direction validation was applied.

Focused acceptance command:

```
TMPDIR=/home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-proof-library pnpm exec vitest run src/views/text/proofLibrary.test.ts src/views/text/useProofLibrary.test.tsx

 Test Files  2 passed (2)
      Tests  14 passed (14)
   Duration  44.59s
```

Local ESLint passed for all four files; Prettier formatted only owned files.
Parent owns final shared frontend check/lint/test after integration. Native
`.fcf`/gzip persistence and history coverage is delegated to the Rust child.

Limits: 256 entries, 32,768 UTF-16 units of proof text per entry, 96-character
identifiers, 128-character names/groups and 64-character script/language labels.
The metadata uses existing native serialization; no model, API or dependency
changes. UI application, translations and interactive browser verification are
owned by the proofing parent. No FontForge source was consulted.

Resource check at task start: 5.23 GiB / 12,405 build files; within budget.

Final focused ESLint exited 0. Owned scratch directory was empty after Vitest
and removed; no task processes remain. Resource check before cleanup passed:
build 6.07 GiB / 13,157 files, aggregate temporary 0.11 GiB / 6,428 files,
668.96 GiB free. Shared growth came from concurrent tasks; no other task's
artifacts were removed. Final Git inspection contains only the intended new
proof-library files within this task's ownership.

Native integration tests were added at `crates/fc-doc/tests/proof_library.rs`.
Two initial focused Cargo attempts were blocked before test execution by
in-flight edits in other crates. The final queued attempt was cancelled at the
root coordinator's request to free build capacity. Native acceptance is pending
the root workspace test run; no Rust test pass is claimed here.
