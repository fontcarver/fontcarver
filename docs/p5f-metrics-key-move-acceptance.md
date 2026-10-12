# Metrics-key scaling helper move

Date: 2026-10-02. Branch: `p5-color-authoring`. Validation recorded before commit.

Completed the code-motion follow-up from task `f` in [TODO](TODO.md).
`fc_doc::metrics::scale_metrics_key` now lives beside `parse_key`, with its two
existing unit tests. `fc-api` delegates to that helper. The function body and
test bodies match the previous revision exactly; only placement, visibility,
the call site and a documentation reference changed.

Validation (exit 0 for every command):

```text
cargo fmt --all -- --check
(no output)

cargo clippy --all-targets -- -D warnings
    Finished `dev` profile [unoptimized] target(s) in 28.78s

cargo test -p fc-doc -p fc-api
    Finished `test` profile [unoptimized] target(s) in 37.43s
```

The test log totals **920 passed, 0 failed, 2 ignored** across 105 test targets,
including doc tests. Relevant output excerpts:

```text
test metrics::metrics_key_tests::reports_a_formula_it_cannot_rewrite ... ok
test metrics::metrics_key_tests::scales_only_the_font_unit_numbers ... ok
rescale_upm.rs:
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
metrics.rs:
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

`git diff --check` passed. No UI, formula grammar, persistence schema or
dependencies changed. Other backlog items remain open; none was started.

Resource checks passed before and after build batches. Reused the ordinary
workspace profiles: build artifacts grew from **6.27 GiB** to **14.16 GiB**,
below the 20 GiB ceiling but above the 10 GiB warning. Retain these reusable
artifacts; before a future broad build, check the budget and clean the dev
profile if projected growth would exceed the ceiling. Tests used an absolute
`TMPDIR` under `.scratch/p5f-metrics-key-move`; their processes finished and the
task scratch directory was removed after retaining this report. No open
questions for this move.
