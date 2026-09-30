# Scripts and plugins

Open **View → Scripts & Plugins** to run a macro. Opening the console, selecting a
file and reloading the scripts list never execute its source. Review the editor
contents and press **Run script** to execute them. The console shows the captured
document, printed output, returned values and errors.

## JavaScript and generated TypeScript declarations

JavaScript macros run in a dedicated worker. They can use top-level `await`,
`return`, `console.log`, `console.info`, `console.warn` and `console.error`.
`fc` and `fontcarver` are aliases for the same host API:

```javascript
const doc = fc.context.doc;
if (doc === null) throw new Error("Open a font first");
const summary = await fc.call("doc.summary", { doc });
console.log(summary);
```

The context captures `doc`, `master` and `glyph` when Run is pressed. Each can
be `null`. `fc.call(method, params)` returns a promise and uses the application's
normal typed transport. Kernel validation, document events and undo behavior
remain in effect. Calls involving a document settle that document's pending
editor drafts before dispatch, including both source and destination for
cross-document operations. Recursive `automation.*` calls and the editor's
low-level `doc.edit_begin`/`doc.edit_apply`/`doc.edit_end` gesture APIs are
unavailable; use `doc.apply` with a labelled command for script edits.

`pnpm -C app gen:api` generates
`app/src/api/generated/fontcarver.d.ts`, along with the complete `ApiMethods`
parameter/result definitions. Add that declaration file to your editor's
TypeScript project, or reference it from a JavaScript file using `@ts-check` and
a triple-slash `reference path`. The generated declarations describe the
`fc`/`fontcarver` globals and infer each method's parameters and result. The
console executes JavaScript; compile TypeScript source to JavaScript first.

Use **Stop script** for a JavaScript loop or a long-running macro. Stop terminates the
worker and prevents new host calls. An API operation already dispatched is
allowed to finish before the console reports that execution has stopped.
Stopping does not undo edits that have already completed. Use the normal Undo
command to reverse their labelled history entries. A sequence of independent
API calls is not a transaction; use a kernel batch command for one undo entry.
JavaScript runs have a 64 KiB source limit, 1 MiB message limit, 256-call limit,
and 64 KiB/256-entry output limit. The default deadline is 10 seconds.

## Local scripts and reload

The console displays its scripts directory inside FontCarver's configuration
directory. Save trusted UTF-8 `.js` or `.py` files directly in that directory,
then press **Reload scripts**. Selecting a script loads it into the editor;
press Run separately. Editing the console source does not overwrite the file.

Discovery lists up to 256 files and examines at most 4,096 directory entries.
Files must be regular files of at most 64 KiB. Nested paths, symlinks and
oversized files are rejected; skipped entries produce bounded diagnostics.
Reload refreshes the cached list and re-reads the selected file if its console
source has not been edited. A manually edited console draft is preserved.

## Python

Select **Python**, enter or load source, and press Run. FontCarver starts an
installed Python interpreter as a separate process and exposes a synchronous
JSON RPC bridge:

```python
import fontcarver

doc = fontcarver.context["doc"]
if doc is None:
    raise RuntimeError("Open a font first")
print(fontcarver.call("doc.summary", {"doc": doc}))
```

The bridge uses the same kernel request/response contract. Writes are limited
to the captured document; session lifecycle, configuration writes, nested
automation and external-process operations are rejected. A failed RPC raises
a Python exception. Normal `print` output and exceptions appear in the console.

The console sets a 10-second deadline. The API accepts 100–30,000 ms, with a
64 KiB source limit, 256 KiB protocol-message limit, 64 KiB captured-output
limit and 1,000-call limit. Only one Python run per API session is accepted at
a time. Python has no interactive Stop operation in this version; it runs to
completion or the deadline. Completed edits remain undoable after an exception
or deadline failure. Interpreter startup failures are reported in the console.
An already-dispatched kernel operation completes before the call returns, even
if the interpreter's deadline expires during that operation.

Run only trusted code. Worker separation keeps JavaScript loops off the UI
thread; it is not a security sandbox. Python runs with the user's filesystem
and process permissions. Deadlines and message limits bound the host protocol,
not all effects of arbitrary code or programs that code launches. The local
development server checks Host and browser Origin before dispatch and must not
be exposed as a remote automation service.

## Bundled plugin manager

Open the plugin manager from the scripts console. Its curated index currently
contains **Bounds reporter** and **Translate filter**. Installing an entry
records it locally in the disabled state; enabling it activates its behavior. Disable removes its
active behavior and Remove also clears the installed entry. These choices
persist in browser/WebView storage. Unknown stored IDs are ignored.

The index contains code shipped with FontCarver. It does not download or run
third-party code, and there is no remote marketplace or external plugin loader.

The Bounds reporter adds an outline bounding box to the glyph canvas while it
is enabled. Reporter plugins receive read-only geometry and return declarative
font-space overlays. They receive neither the canvas context nor document
mutation access. Disabling the reporter removes its overlay without changing
the font.

The Translate filter targets the active editable master layer. Enter X/Y
offsets and request a preview. The preview is transient and does not create an
undo entry. Apply uses the exact captured request and the preview's document
generation; a changed document requires a new preview. Apply creates a normal
undoable kernel change. Cancel discards the preview. Translation uses the
existing style pipeline and can bake components or live effects into outlines;
review the displayed scope before applying it.
Closing the manager, changing glyph or disabling/removing the plugin discards
pending previews. An apply operation already dispatched finishes normally;
cancel does not recall it.

Host contracts live in `app/src/plugins/contracts.ts`: `PluginManifest`,
`ReporterPlugin`, `FilterPlugin`, `PluginOverlay` and `FilterPlan`. To add a
bundled implementation, provide metadata and a pure reporter or filter-plan
function, add it to the curated registry, and add English display strings and
tests. A filter plan uses an existing revision-checked kernel operation; the
host owns preview, stale checking, apply, cancellation and lifecycle cleanup.
