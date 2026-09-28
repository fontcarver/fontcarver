# Workspace instructions

Read and follow [CONVENTIONS.md](CONVENTIONS.md) before making changes.

Disk usage is part of task completion. Follow its **Build caches and temporary
artifacts** rules at task start, around broad builds/tests, and before reporting
each small task complete. Run `python3 scripts/check_artifacts.py`; an exceeded
budget must be addressed before more builds. Never delete source files, user
fonts, Git data, shared dependency stores, or another task's files as cleanup.

Ordinary development and tests use the space-saving Cargo profiles in
`Cargo.toml`. Do not enable full debug information or incremental compilation
without a specific debugging need and a plan to remove those extra artifacts.
