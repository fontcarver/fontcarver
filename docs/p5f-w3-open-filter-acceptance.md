# Native Open Font filter acceptance — 2026-10-02

The native Open Font filter hid nine formats already accepted by `session.open`.
Added `cff`, `t42`, `p42`, `type42`, `svg`, `eps`, `afm`, `pfm` and `ttx` to
`app/src/shell/nativeDialogs.ts`. No codec, backend, locale or dependency changed.
The API's explicit opening branches were checked for every added extension.

The existing boundary test now checks all supported interchange choices; four
cases also verify that CFF and Type 42 alias selections reach the shared document
opener. Before the filter change, these assertions produced **5 failed / 8 passed**.

```text
TMPDIR=$PWD/.scratch/p5f-w3-open-filter/tmp pnpm --dir app exec vitest run \
  --maxWorkers=1 --minWorkers=1 src/shell/nativeDialogs.test.ts
Test Files 1 passed (1)
Tests 13 passed (13)
Duration 1.99s

pnpm --dir app exec eslint src/shell/nativeDialogs.ts src/shell/nativeDialogs.test.ts
exit 0
pnpm --dir app exec prettier --check src/shell/nativeDialogs.ts src/shell/nativeDialogs.test.ts
All matched files use Prettier code style!
git diff --check -- app/src/shell/nativeDialogs.ts app/src/shell/nativeDialogs.test.ts
exit 0
```

`pnpm --dir app check` exposed TS2352 in the separate CJK regression fixture
this audit agent had just handed off (`CjkTransformGrid.test.tsx:356`, incomplete
EditorController mock cast). No filter-specific finding. The CJK owner was
notified to correct the fixture and include TypeScript verification before final
integration; this failed command is not claimed passed here.

Independent reviewer `w3_review_release` accepted the two-file change and reran
**13/13 tests** (1.71 s). This verifies emitted filter and selected-path routing
at the mocked Tauri boundary, not native picker execution.

No owned process remains; task scratch was removed. Final artifact check:

```text
build: 15.88 GiB, 19,638 files
temporary: 0.01 GiB, 586 files
workspace free: 606.08 GiB
warnings: build artifacts exceed 10 GiB; plan profile-cache cleanup
PASS
```
