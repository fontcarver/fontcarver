# Final P1 cleanup

All 14 temporary worktrees were checked for a clean tracked/untracked status,
merged ancestry and running processes referencing their path before removal.
All passed and were removed using git worktree remove without force. Branch
refs and the main working directory were retained; no user process was stopped.

The final desktop executable was preserved at root target/p1-acceptance before
cleanup. Release CLI/server artifacts remain in root target/release. Final
reports and text reproductions are snapshotted to tracked docs; the ignored
ledger (including temporary generated review fonts/executable) is then removed.
The snapshot script now copies text evidence only. A newly created local snapshot
was corrected immediately to exclude its accidentally included compiled probe;
no pre-existing project history or historical fixture purge was changed.

Removed worktrees:

- p1-acceptance-browser
- p1-acceptance-kernel
- p1-cjk-acceptance
- p1-desktop-acceptance
- p1-http-timeouts
- p1-t7-minors
- p1-t8
- p1-t9b
- p1-text-controls
- p1-text-kernel
- p1-text-metrics
- p1-text-ui
- p1-zh-cn
- p1-zh-text
