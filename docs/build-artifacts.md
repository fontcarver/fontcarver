# Build artifact storage

On 2026-09-28 the workspace occupied approximately 493 GiB. A filesystem scan
located almost all of it under `target/debug`: approximately 393 GiB of dependency
and test artifacts, 93 GiB of incremental caches, and 5 GiB of other development
outputs. The source tree, Git data and frontend dependencies were below 1 GiB.

The workspace previously used Cargo's default development/test profiles, including
debug information and incremental compilation. Many separately linked integration
tests and repeated build configurations accumulated without a storage check or
cleanup checkpoint. This was uncontrolled development output, not the installed
application's storage requirement. The measured directory breakdown does not
separately quantify stale artifacts versus debug information within each binary.

The active test and development-server processes were stopped. After inspecting
the paths and running a dry run, `cargo clean --profile dev` removed the disposable
development artifacts. Approximately 491 GiB of allocated disk space was released:
the workspace fell to 2.1 GiB and filesystem free space rose from 205 GiB to
696 GiB. Release outputs and historical acceptance artifacts remained (about
1.55 GiB under `target`, 3,511 files). Source files, uncommitted P5-10 work, Git
history and font fixtures were preserved. The interrupted P5-10 Rust regression
run is not recorded as a full pass.

The resource rules are in [CONVENTIONS.md](../CONVENTIONS.md), referenced by the
workspace [AGENTS.md](../AGENTS.md). Default development/test profiles omit debug
information and incremental caches. The read-only
[`check_artifacts.py`](../scripts/check_artifacts.py) reports allocated bytes,
file counts, free space and budget failures. It is called by CI and the developer
launcher and is required at task/build checkpoints. It never deletes files.
These checkpoints are not a continuous filesystem quota.

Every small task removes its own superseded scratch exports, fonts, browser
profiles and traces after retaining the necessary acceptance evidence. Shared
build artifacts are reused within the 20 GiB / 100,000-file budget, with a 10 GiB
warning. Task temporary output has a 1 GiB / 10,000-file budget. The local free-space
floor is 20 GiB; disposable CI runners use 5 GiB. An optional debugger build must
include cleanup of the extra profile artifacts.

Validation of the maintenance change: five Python checks passed (hard-link
accounting, symlink boundaries, budget failure without deletion, free-space failure,
invalid limits); Cargo metadata accepted the profiles; Rust formatting and diff
whitespace checks passed. The resource check passed with the ongoing P5-10 and
carried development-server acceptance directories included: 1.55 GiB of build
artifacts and 0.18 GiB of temporary output. A first build after cleanup must rebuild
its dependencies and will be slower; no full rebuild was needed to validate the
read-only resource checker.
