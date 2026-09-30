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

The console ships those declarations with the application. Its **API reference**
tab shows the declaration file's path and full text, copies it to the clipboard,
and searches every kernel method by name or parameter key. Each result lists the
method's parameter and result types and its accepted keys, and **Insert call**
appends a ready-to-run `await fc.call(...)` with every required parameter.

Before a JavaScript run, the console checks each literal `fc.call` against the
same generated contract:

- an unknown method, or one the scripting facade withholds, is an **error** and
  Run stays disabled until you correct it;
- an unknown parameter key is a **warning**, because the kernel rejects it
  anyway and the key scan is deliberately conservative.

A *missing* key is never reported: many kernel parameters have a default, which
the generated TypeScript does not distinguish from a required field.

A call whose method name or parameter object is computed — a variable, a spread
or a computed key — is left unchecked rather than guessed at, and a call written
inside a comment or a string is never reported. The check is not a TypeScript
compiler; use the shipped declarations in your editor for full type checking.

## One run, one undo entry

A run that captures a document opens a history transaction for it, so every
change the script makes — however many kernel calls it takes — collapses into a
single undo entry named after the script, such as `Script: widen.js`. The
console reports the recorded entry when the run finishes. Undo reverses the
whole run; redo reapplies it.

Because the pending entry covers the whole run, `doc.undo` and `doc.redo` are
rejected **inside** a run with an explicit error. Undo the run after it ends
instead. A run that fails part-way still records the work it completed, so one
undo restores the document. Saving during a run does not split the entry.

A transaction cannot outlive its run. It carries a deadline — a minute for a
console run — after which the next edit, gesture, undo, redo or save closes it,
and starting another run closes any transaction an abandoned one left behind.
So a reloaded page or a dropped connection costs at most that one entry; later
edits are recorded separately and undo keeps working.

Use **Stop script** for a JavaScript loop or a long-running macro. Stop terminates the
worker and prevents new host calls. An API operation already dispatched is
allowed to finish before the console reports that execution has stopped.
Stopping does not undo edits that have already completed; the run's transaction
is closed, so one Undo reverses everything it managed to change.
JavaScript runs have a 64 KiB source limit, 1 MiB message limit, 256-call limit,
and 64 KiB/256-entry output limit. The default deadline is 10 seconds.

## Local scripts, folders and reload

The console searches three script folders and lists each one with its path:

| Folder                 | Where it comes from                                              |
| ---------------------- | ----------------------------------------------------------------- |
| **Configured folder**  | The absolute path you enter in *Extra scripts folder* and save.   |
| **Preferences folder** | `scripts` inside FontCarver's configuration directory.            |
| **Project folder**     | `scripts` beside the active document's file or project directory. |

Only the preferences folder is created for you; the other two are listed as
*not present* until they exist. Names are resolved inside the folder a script
was listed from, so two folders may hold files with the same name. Save trusted
UTF-8 `.js` or `.py` files in any of them. Selecting a script loads it into the
editor; press Run separately. Editing the console source does not overwrite the
file.

Discovery lists up to 256 files per search and examines at most 4,096 directory
entries. Files must be regular files of at most 64 KiB. Nested paths, symlinks
and oversized files are rejected; skipped entries produce bounded diagnostics.

**Reload scripts** refreshes the list on demand and re-reads the selected file
if its console source has not been edited; a manually edited console draft is
preserved. While the console is open it also **reloads on change**: it re-reads
the searched folders every two seconds and refreshes the list when their
contents change, so a script saved from an editor appears without pressing
Reload. Reading a folder never executes a script.

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

A Python run with a captured document is also one history transaction, so the
whole run is a single undo entry, named by the `label` the console sends.

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

Open the plugin manager from the scripts console. Its curated index is the
bundled JSON file `app/src/plugins/index.json`, and each entry carries its
provenance: author, licence and the repository path of the shipped
implementation. The manager shows those values beside every plugin. The index
currently contains **Bounds reporter**, **Metrics reporter** and **Translate
filter**.

Installing an entry records it locally in the disabled state; enabling it
activates its behavior. Disable removes its active behavior and Remove also
clears the installed entry. **Reload plugins** re-reads the curated index and
the stored installation record — useful after another window changed it — and
never downloads or re-evaluates code, because every implementation is compiled
into the application. These choices persist in browser/WebView storage together
with each plugin's settings. Unknown stored IDs, versions and settings are
ignored or repaired against the index.

An entry may declare numeric or toggle **settings**. The manager shows them
below an installed plugin, clamps each value to its declared range and step,
and persists it. The Translate filter's X/Y offsets are settings, so they are
restored the next time you open the manager.

The index contains code shipped with FontCarver. It does not download or run
third-party code, and there is no remote marketplace or external plugin loader.

The Bounds reporter draws the outline bounding box on the glyph canvas, and
optionally the advance width. The Metrics reporter draws the baseline and,
optionally, the x-height and cap-height lines across exactly the visible part of
the canvas. Reporter plugins receive read-only geometry, a document context
(document, glyph, layer, master and the resolved layer metrics), a view context
(zoom and the visible font-space rectangle) and their persisted settings, and
return declarative font-space overlays. They receive neither the canvas context
nor document mutation access. Disabling a reporter removes its overlay without
changing the font. Overlays are not drawn while the glyph tab is showing a text
run rather than an editable glyph.

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
`PluginIndexEntry`, `PluginSetting`, `ReporterPlugin`, `ReporterInput`,
`FilterPlugin`, `PluginOverlay` and `FilterPlan`. To add a bundled
implementation, write a pure reporter or filter-plan function in
`app/src/plugins/`, add an entry with its provenance and settings to
`app/src/plugins/index.json`, and add English display strings and tests. An
entry whose implementation, provenance or settings do not validate is dropped
from the index rather than offered for installation. A filter plan uses an
existing revision-checked kernel operation; the host owns preview, stale
checking, apply, cancellation and lifecycle cleanup.
