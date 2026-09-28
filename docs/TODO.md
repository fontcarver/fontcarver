# Resume checklist

Paused by explicit user request on 2026-09-28 after completing P5-10 smart
corners. The next session must wait for the user's instruction to resume.
The branch is `p5-color-authoring`; changes are saved locally, with no push or
main merge. Native/global desktop input automation remains prohibited.

## Saved milestones

- [x] P5-09: persistent layer effects and stored masks; [acceptance](p5-layer-effects-acceptance.md).
- [x] P5-10: numeric live smart corners and ink traps; [acceptance](p5-smart-corners-acceptance.md).
- [x] Resource controls: cleared approximately 491 GiB of stale development
  artifacts, changed Cargo profiles, added task cleanup rules and CI/developer
  checks; [record](build-artifacts.md).
- [x] Preserve final reports/screenshots, remove this task's scratch output,
  stop its browser/development services and leave the working tree clean.

## First work after resuming

1. Read [STATUS](STATUS.md), [AGENTS.md](../AGENTS.md) and
   [CONVENTIONS.md](../CONVENTIONS.md); run `python3 scripts/check_artifacts.py`.
2. Scope P5-11 around **variable pen points**: source positions, width, height,
   angle and progression on top of the existing uniform live stroke evaluator.
   Write its task/acceptance plan before implementation. Row 87 stays partial
   until the complete editing, persistence and export path is verified.
3. Continue the remaining work below. The future `P5-NN` count is not fixed;
   the original plan has fourteen broad work blocks and the exit audit has 112
   numbered requirements. A task number is not a completion percentage.

## Remaining P5 work

| Work | Remaining scope |
| --- | --- |
| Hinting | Variable hinting, flex and per-FD private data; remaining TT/PS acceptance boundaries. |
| Geometry and construction | Variable pen points; corner/cap/segment/brush components; node-linked guides; servant nodes; image tracing/background images; optical spacing/kerning; remaining CJK/editor/variable requirements. |
| Corner interaction follow-up | Dedicated radius drag handles and construction at crossings without a node; any broader reference behavior beyond P5-10's recorded bounds. |
| Document workflows | Compare/family compare, merge, paste special, cross-document copy, visual diffs and reference binary handling. |
| Automation | Scripts console/reload, Python RPC, plugin lifecycle/manager, reporter/filter contracts and documentation. |
| Formats/export | Remaining SVG/EPS/AFM/PFM/TTX/collections, preprocessing, profiles, preflight and shared CLI behavior. |
| Shell and UX | Windows/workspaces, filters, metadata, proofing, shortcuts, accessibility/help and remaining feature/preview gaps. |
| Translation review | Independent review of the implemented Chinese strings; implementer browser checks do not replace that review. |
| Release | Packaging definitions, locally verifiable installers, platform CI and user/developer documentation. |
| Final acceptance | Verify every Must-have/Should-have audit row with working behavior and evidence; native-platform/package claims remain unverified. |

The authoritative completion gate is [p5-feature-audit.md](p5-feature-audit.md).
Its pending rows can include already implemented P1–P4 behavior awaiting scoped
verification. No whole-project percentage or firm remaining duration is claimed.
