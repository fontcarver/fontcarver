# P5 resumed implementation coordination

Resumed on 2026-09-30 by the user's instruction to continue all remaining work
without confirmation. The user explicitly requested extensive parallel agent
work. Earlier pause instructions are superseded; the existing prohibition on
native/global desktop input automation remains in force.

## Latest user instruction: finish underway work, then stop

The user subsequently requested finishing tasks already in progress and then
stopping to rest. The current parallel batch is frozen: complete its necessary
implementation, integration, verification and cleanup; do not start further
scope or new delegations beyond fixes required to complete this batch. Record
remaining gaps in TODO, save a truthful checkpoint and stop after acceptance.

## Shared rules

Read `AGENTS.md` and `CONVENTIONS.md`. Work in the shared checkout; never reset,
revert, delete, or overwrite another task's changes. Do not commit or push.
Write a bounded task plan before implementation and an acceptance report with
actual evidence afterward. Parent maintains STATUS, TODO, progress estimate and
the feature audit; report recommended row updates without editing those files.

Use a task-specific `.scratch/p5-<task>/` directory and an absolute TMPDIR for
tests. Check `python3 scripts/check_artifacts.py` at task boundaries. Share the
workspace Cargo target directory and normal profiles. Serialize Cargo commands
with `flock /home/chen/githubprojects/fontpixel/fontcarver/.scratch/p5-resume/cargo.lock`.
Run focused tests while developing; the parent runs final workspace Clippy,
regression suites and frontend checks after integration. Do not run broad
formatters that rewrite other agents' files. Coordinate edits to registries,
model structs and API dispatch with their current owners; use narrow patches.
Run focused Vitest commands with `--maxWorkers=1 --minWorkers=1`; the shared
default is capped at two workers so separate agents do not each launch a pool
proportional to every CPU on the host. Do not repeat Cargo checks against known
incomplete shared modules: report the owner error, continue independent work,
and retry after the owner or Rust integration agent confirms it is fixed.

All new UI strings go in English. The independent localization agent owns
`app/src/i18n/zh-CN.json`; feature agents must not write translations. Notify the
parent of new key namespaces. Never hand-edit generated API files; coordinate
generation after Rust APIs compile. New dependencies need permissive licenses
and a justification/provenance entry. Use agent-browser for isolated headless
visual verification, never native/global desktop input.

Deliver working integrated behavior, failure/undo/persistence coverage and
honest acceptance boundaries. Do not mark a checklist row complete based on a
helper, placeholder UI or build alone. If a shared-file conflict or external
platform blocks you, report the exact issue and continue independent work.

## Parallel areas

- Variable pen geometry/model and its separate UI editing path (P5-11).
- Cross-document paste/relink/merge and comparison.
- Script execution, Python RPC and plugin contracts.
- Remaining interchange formats and export profiles/preflight/CLI.
- Shell workspaces, shortcut editing/help and proofing.
- Glyph inventory/filter workflows and outline editor parity.
- Linked construction, image workflows and optical metrics.
- Variable/CJK workflows, bitmap parity and feature lookup authoring.
- Hinting, packaging, independent translation and audit reconciliation.

Each owner records a finer task boundary before editing. Milestones are accepted
only after integration checks. Native macOS/Windows execution cannot be claimed
from Linux build definitions or simulated platform checks.

## Final stopping point

The frozen batch finished integration and stopped on 2026-09-30. See
[central results](../../../p5-resume-integration.md). All delegated owners returned;
no further feature work or agents were started during final integration. Root
fixed the remaining regressions, retained evidence and removed its scratch.
