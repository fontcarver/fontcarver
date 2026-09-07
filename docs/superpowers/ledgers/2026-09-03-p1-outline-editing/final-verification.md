# Final P1 verification and stop boundary

Date: 2026-09-07. Production candidate: `2e53ba6432b9bee3aa15000a7918f472c9289fa9`.
Subsequent changes are documentation and the snapshot artifact filter.

- App TypeScript, full ESLint and Prettier passed.
- Full Vitest: 104 files, 930 tests passed in 10.76 s.
- Fresh release-kernel headless Playwright, ports 8787/1428, one worker:
  21 passed, no skips, 40.9 s. Actual Source Han grid workflow passed in 1.7 s.
- Browser DejaVu edit/export/reopen 3.3 s; scratch A-C pen/width/export/reopen
  3.5 s. Text shape/local editing/metrics undo/two Escape stages passed.
- 300-point drag: 20 warmup + 200 measured frame-paced moves; median 0.9 ms,
  p95 1.5 ms, max 5.2 ms, zero over 8 ms; pointer-down 2.4 ms. Exact persisted
  coordinates and undo passed after asynchronous edit_end completed.
- Final workspace Rust formatting and all-target Clippy passed. Prior full
  integrated Rust run passed 453 tests; final nine preservation tests passed
  independently after the TTC correction.
- Independent final review approved F1/F2/F3; 57 scoped app tests passed.
- Final release CLI/server build passed in 186.30 s. Embedded debug desktop
  build passed in 16.03 s; preserved root target/p1-acceptance artifact was never
  launched. Native final interaction and release packaging remain unverified.

Earlier contended view-test failures, the five-second drag-flush timeout and
its cleanup NotFound messages are retained as diagnostics, not acceptance passes.
No assertions or 8 ms timing bounds were loosened. All remaining verification
was headless after the user's input-interference complaint.

P1 implementation, translation and authorized headless acceptance are complete.
Stop now; no P2 and no main merge. See docs/p1-acceptance.md for qualified
preservation, preview and Source Han timing limitations.
