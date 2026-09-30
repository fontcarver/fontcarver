# P5 automation checkpoint — 2026-09-30

Scope: feature-audit rows 79, 105 and 106. Implementation is frozen following
the user's request to finish work already underway and stop. This checkpoint
does **not** claim completed backend or browser acceptance.

## Implemented

- **Scripts & Plugins** console, explicit JavaScript/Python execution, source
  editor, output/errors, local scripts folder, cached discovery and reload.
  Reload refreshes unchanged selected source without executing it or replacing
  edited console drafts.
- Dedicated JavaScript worker, execution/message/output/call bounds, Stop,
  draining of dispatched API work and cancellation of calls still waiting for
  document drafts. Cross-document RPC settles both source and destination.
  Caught API failures can be handled by the script. Recursive automation and
  raw editor gesture-transaction calls are rejected.
- Generated `fontcarver.d.ts` scripting globals derived from the kernel method
  contract. Generation excludes the declaration from runtime barrel exports.
- `automation.scripts_list`, `scripts_reload`, `script_read` and `python_run`.
  Script discovery/read reject traversal and file symlinks. Python uses an
  installed interpreter, UTF-8 source and bounded line-oriented JSON RPC, with
  one concurrent run, a watchdog and captured-document mutation restrictions.
- Curated bundled plugin index with persistent explicit install/enable/disable/
  remove. Bounds reporter draws through a declarative overlay contract on the
  glyph canvas. Translate filter shows a non-mutating preview and applies the
  captured `style.batch` request with an atomic document-generation check.
- Development API Host/Origin validation before HTTP/WebSocket dispatch.
  Release owner added the CSP required for the static macro worker and Blob
  script module without enabling `unsafe-eval`.

User/runtime documentation: [automation guide](../user/automation.md).

## Evidence collected

| Focused verification | Result |
| --- | --- |
| Console/store/bridge Vitest suites | 13 passed |
| Worker/runtime Vitest suite | 28 passed |
| Plugin lifecycle Vitest suite | 24 passed |
| Plugin canvas overlay Vitest suite | 16 passed |
| Filter lifecycle Vitest suite | 20 passed |
| Plugin manager UI Vitest suite | 7 passed |
| Automation/plugin scoped ESLint and formatting | Passed |
| Worker/runtime focused TypeScript check | Passed |
| Rust changed-file formatting, Python source parse | Passed |
| Independent runtime review | Two frontend defects corrected and regression tested; no backend/Origin blocker found within the documented trusted-code boundary |

Total focused frontend tests: **108 passed**. A prior console UI run hit its
5-second timeout under machine contention; the final serialized rerun passed.
A shared frontend type check exited 2 while generated contracts and concurrent
implementations were incomplete. Four plugin test inference errors from that
run were corrected and the affected focused suite reran successfully. This
does not substitute for the final generated-API/full-app check.

## Centralized acceptance still required

The root task canceled queued Cargo work and prohibited new browser profiles
when machine load and temporary-file counts became excessive. Therefore:

- The 20 new backend integration tests in `crates/fc-api/tests/automation.rs`
  are **not run**: their queued command was canceled before compilation,
  exit 143, with an empty log.
- The two new development-server Host/Origin tests are **not run**: the
  attempted focused command stopped at concurrent `fc-model` compilation
  errors before reaching this package. The diagnostic is retained next to
  this report until root integration resolves it.
- The new scripting-declaration generator test needs the centralized Rust
  test run, followed by regeneration and the final app type/lint/test/build
  checks.
- No headless browser acceptance session was created by this owner. Verify
  console JavaScript output/error/Stop, disk script reload, Python query and
  undoable mutation, reporter enable/disable, filter preview/cancel/apply/Undo
  and persistence after application reload against the rebuilt server.
- Native Tauri worker execution on Linux/macOS/Windows is not claimed from
  the CSP configuration alone.

Required Rust commands include `cargo test -p fc-api --test automation`,
`cargo test -p fc-api --bin gen_ts`, and `cargo test -p fc-devserver --test http`.
Root owns serialized API generation and final workspace checks.

## Boundaries

Scripts are explicitly executed trusted local code. Worker/process separation
is not an OS sandbox. Python has deadline-based termination, no interactive
Stop; an already-dispatched kernel call can finish after the deadline.
Intentionally launched descendant processes are not managed. Completed script
edits remain normal undo entries; an entire script is not a transaction.

The plugin index contains two shipped implementations and does not download
third-party code. Filters currently target the active master layer through the
existing style pipeline, which can bake components/live effects. Generated
TypeScript declarations provide editor types; the console executes JavaScript,
so TypeScript source must be compiled first. External plugin distribution and
a general-purpose third-party extension loader are not implemented.

No dependencies, commits or Chinese translations were added by this team.
English namespaces `automation` and `plugins` are ready for the independent
translation pass. Owned processes have stopped; task scratch is cleaned after
retaining this report and the single unresolved compilation diagnostic.
