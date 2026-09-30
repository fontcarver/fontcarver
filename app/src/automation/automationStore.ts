import { create } from 'zustand'

import {
  call,
  type AutomationScriptEntry,
  type AutomationScriptFolder,
  type AutomationScriptFolderKind,
} from '../api'
import { withSettledDocument } from '../editor/documentPersistence'
import { currentMacroContext, macroCall } from './macroBridge'
import { runJavaScript, type MacroContext } from './runtime'
import { checkScript, type ScriptProblem } from './scriptCheck'

/** A displayed log entry from either scripting runtime. */
export interface ConsoleOutput {
  level: 'log' | 'info' | 'warn' | 'error' | 'result'
  text: string
}

type Language = 'javascript' | 'python'
type Status =
  'idle' | 'running' | 'stopping' | 'completed' | 'stopped' | 'failed'

/** Interval between script-folder change checks while the console is open. */
export const WATCH_INTERVAL_MS = 2000

/**
 * Lifetime the kernel gives a run's history transaction.
 *
 * A JavaScript run stops after ten seconds and its dispatched kernel calls
 * settle after that, so this is generous; it exists so that a reloaded page or
 * a lost transport cannot leave history open indefinitely.
 */
export const TRANSACTION_LIFETIME_MS = 60_000

interface AutomationData {
  open: boolean
  source: string
  language: Language
  selectedScript: string
  selectedFolder: AutomationScriptFolderKind
  folders: AutomationScriptFolder[]
  scripts: AutomationScriptEntry[]
  fingerprint: string
  /** Extra folder from the console's scripting preference; empty when unset. */
  configuredDirectory: string
  warnings: string[]
  loaded: boolean
  loading: boolean
  reading: boolean
  running: boolean
  status: Status
  output: ConsoleOutput[]
  error: string | null
  context: MacroContext | null
  problems: ScriptProblem[]
  /** Set when a folder changed on disk and the list was refreshed for it. */
  changedOnDisk: boolean
  /** Undo label of the last run's single history entry. */
  historyLabel: string | null
}

interface AutomationState extends AutomationData {
  setSource(source: string): void
  setLanguage(language: Language): void
  loadScripts(reload?: boolean): Promise<void>
  openScript(name: string, folder: AutomationScriptFolderKind): Promise<void>
  /** Resolves to whether the folder was accepted and stored. */
  setConfiguredDirectory(path: string): Promise<boolean>
  checkFolders(): Promise<void>
  run(label: string): Promise<void>
  stop(): void
  close(): void
  clearOutput(): void
  insertSnippet(snippet: string): void
}

let sourceRequest = 0
let cancelRun: (() => void) | undefined

/** The serializable initial console state, also used by isolated tests. */
export function initialAutomationState(): AutomationData {
  return {
    open: false,
    source: 'console.log(await fc.call("session.list", {}))',
    language: 'javascript',
    selectedScript: '',
    selectedFolder: 'preferences',
    folders: [],
    scripts: [],
    fingerprint: '',
    configuredDirectory: '',
    warnings: [],
    loaded: false,
    loading: false,
    reading: false,
    running: false,
    status: 'idle',
    output: [],
    error: null,
    context: null,
    problems: [],
    changedOnDisk: false,
    historyLabel: null,
  }
}

const errorText = (error: unknown) =>
  error instanceof Error ? error.message : String(error)

const problemsFor = (source: string, language: Language) =>
  language === 'javascript' ? checkScript(source) : []

/** Console editing, local discovery and explicit runtime lifecycle. */
export const useAutomationStore = create<AutomationState>((set, get) => {
  const append = (entry: ConsoleOutput) =>
    set((state) => ({
      output: [
        ...state.output,
        { ...entry, text: entry.text.slice(0, 8192) },
      ].slice(-250),
    }))

  const listParams = () => ({ doc: currentMacroContext().doc })

  return {
    ...initialAutomationState(),
    setSource: (source) => {
      if (get().running) return
      sourceRequest += 1
      set({
        source,
        selectedScript: '',
        reading: false,
        problems: problemsFor(source, get().language),
      })
    },
    insertSnippet: (snippet) => {
      if (get().running) return
      sourceRequest += 1
      const previous = get().source
      const source =
        previous.length === 0 || previous.endsWith('\n')
          ? `${previous}${snippet}\n`
          : `${previous}\n${snippet}\n`
      set({
        source,
        selectedScript: '',
        reading: false,
        problems: problemsFor(source, get().language),
      })
    },
    setLanguage: (language) => {
      if (get().running) return
      sourceRequest += 1
      set({
        language,
        selectedScript: '',
        reading: false,
        problems: problemsFor(get().source, language),
      })
    },
    loadScripts: async (reload = false) => {
      if (get().loading || (get().loaded && !reload)) return
      set({ loading: true, error: null })
      try {
        const [result, settings] = await Promise.all([
          call(
            reload ? 'automation.scripts_reload' : 'automation.scripts_list',
            listParams(),
          ),
          call('automation.settings_get', {}),
        ])
        const { selectedScript, selectedFolder } = get()
        set({
          ...result,
          configuredDirectory: settings.scripts_directory ?? '',
          loaded: true,
          changedOnDisk: false,
        })
        if (
          reload &&
          selectedScript &&
          result.scripts.some(
            (script) =>
              script.name === selectedScript &&
              script.folder === selectedFolder,
          )
        )
          await get().openScript(selectedScript, selectedFolder)
      } catch (error) {
        set({ error: errorText(error) })
      } finally {
        set({ loading: false })
      }
    },
    checkFolders: async () => {
      const state = get()
      if (!state.open || state.running || state.loading || !state.loaded) return
      try {
        // Discovery is a directory read, never an execution; only a changed
        // fingerprint triggers the reload the user would otherwise request.
        const probe = await call('automation.scripts_reload', listParams())
        if (probe.fingerprint === get().fingerprint) {
          set({ folders: probe.folders, scripts: probe.scripts })
          return
        }
        set({ changedOnDisk: true, loaded: false })
        await get().loadScripts(true)
        set({ changedOnDisk: true })
      } catch {
        // A transient discovery failure must not disturb an open console.
      }
    },
    openScript: async (name, folder) => {
      if (get().running) return
      const request = ++sourceRequest
      set({ open: true, reading: true, error: null })
      try {
        const result = await call('automation.script_read', {
          name,
          folder,
          doc: currentMacroContext().doc,
        })
        if (request !== sourceRequest) return
        set({
          source: result.source,
          language: result.language,
          selectedScript: result.name,
          selectedFolder: result.folder,
          problems: problemsFor(result.source, result.language),
        })
      } catch (error) {
        if (request === sourceRequest) set({ error: errorText(error) })
      } finally {
        if (request === sourceRequest) set({ reading: false })
      }
    },
    setConfiguredDirectory: async (path) => {
      if (get().running) return false
      set({ error: null })
      try {
        const settings = await call('automation.settings_set', {
          scripts_directory: path.trim() === '' ? null : path.trim(),
        })
        set({
          configuredDirectory: settings.scripts_directory ?? '',
          loaded: false,
        })
        await get().loadScripts(true)
        return true
      } catch (error) {
        // A rejected folder keeps the user's text so they can correct it.
        set({ error: errorText(error) })
        return false
      }
    },
    run: async (label) => {
      const { source, language, running, reading } = get()
      if (running || reading || !source.trim()) return
      const problems = problemsFor(source, language)
      set({ problems })
      if (problems.some((problem) => problem.severity === 'error')) {
        set({ status: 'failed' })
        return
      }
      const context = currentMacroContext()
      set({
        running: true,
        status: 'running',
        output: [],
        error: null,
        context,
        historyLabel: null,
      })
      try {
        if (language === 'python') {
          // Python owns a bounded interpreter process. Its transport has no
          // cancellation operation; the UI clearly exposes the 10 s deadline.
          const invoke = () =>
            call('automation.python_run', {
              source,
              ...context,
              timeout_ms: 10000,
              label,
            })
          const result =
            context.doc === null
              ? await invoke()
              : await withSettledDocument(context.doc, invoke)
          for (const text of result.output) append({ level: 'log', text })
          if (result.error) append({ level: 'error', text: result.error })
          set({
            status: result.error ? 'failed' : 'completed',
            historyLabel: result.undo_label,
          })
        } else {
          // One run is one undo entry: every kernel change the macro makes joins
          // this transaction, which is always closed below.
          const transaction =
            context.doc === null
              ? null
              : await call('automation.transaction_begin', {
                  doc: context.doc,
                  label,
                  timeout_ms: TRANSACTION_LIFETIME_MS,
                })
          try {
            const run = runJavaScript(source, context, {
              call: macroCall,
              onOutput: append,
            })
            cancelRun = run.stop
            const result = await run.done
            if (result.error) append({ level: 'error', text: result.error })
            set({ status: result.status })
          } finally {
            // Closing must never mask the run's own outcome, and the kernel
            // reaps the transaction on its deadline if this call never lands.
            if (transaction)
              try {
                const ended = await call('automation.transaction_end', {
                  doc: transaction.doc,
                  token: transaction.token,
                })
                set({ historyLabel: ended.undo_label })
              } catch (error) {
                append({ level: 'warn', text: errorText(error) })
              }
          }
        }
      } catch (error) {
        append({ level: 'error', text: errorText(error) })
        set({ status: 'failed' })
      } finally {
        cancelRun = undefined
        set({ running: false })
      }
    },
    stop: () => {
      if (!get().running || get().language !== 'javascript') return
      set({ status: 'stopping' })
      cancelRun?.()
    },
    close: () => {
      if (get().running) return
      sourceRequest += 1
      set({ open: false, reading: false })
    },
    clearOutput: () => {
      if (!get().running)
        set({ output: [], status: 'idle', error: null, historyLabel: null })
    },
  }
})

/** Opens the console and discovers local scripts without executing any source. */
export function openMacroConsole(): void {
  useAutomationStore.setState({ open: true })
  void useAutomationStore.getState().loadScripts()
}
