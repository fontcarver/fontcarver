import { create } from 'zustand'

import { call, type AutomationScriptEntry } from '../api'
import { withSettledDocument } from '../editor/documentPersistence'
import { currentMacroContext, macroCall } from './macroBridge'
import { runJavaScript, type MacroContext } from './runtime'

/** A displayed log entry from either scripting runtime. */
export interface ConsoleOutput {
  level: 'log' | 'info' | 'warn' | 'error' | 'result'
  text: string
}

type Language = 'javascript' | 'python'
type Status =
  'idle' | 'running' | 'stopping' | 'completed' | 'stopped' | 'failed'

interface AutomationData {
  open: boolean
  source: string
  language: Language
  selectedScript: string
  directory: string
  scripts: AutomationScriptEntry[]
  warnings: string[]
  loaded: boolean
  loading: boolean
  reading: boolean
  running: boolean
  status: Status
  output: ConsoleOutput[]
  error: string | null
  context: MacroContext | null
}

interface AutomationState extends AutomationData {
  setSource(source: string): void
  setLanguage(language: Language): void
  loadScripts(reload?: boolean): Promise<void>
  openScript(name: string): Promise<void>
  run(): Promise<void>
  stop(): void
  close(): void
  clearOutput(): void
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
    directory: '',
    scripts: [],
    warnings: [],
    loaded: false,
    loading: false,
    reading: false,
    running: false,
    status: 'idle',
    output: [],
    error: null,
    context: null,
  }
}

const errorText = (error: unknown) =>
  error instanceof Error ? error.message : String(error)

/** Console editing, local discovery and explicit runtime lifecycle. */
export const useAutomationStore = create<AutomationState>((set, get) => {
  const append = (entry: ConsoleOutput) =>
    set((state) => ({
      output: [
        ...state.output,
        { ...entry, text: entry.text.slice(0, 8192) },
      ].slice(-250),
    }))

  return {
    ...initialAutomationState(),
    setSource: (source) => {
      if (get().running) return
      sourceRequest += 1
      set({ source, selectedScript: '', reading: false })
    },
    setLanguage: (language) => {
      if (get().running) return
      sourceRequest += 1
      set({ language, selectedScript: '', reading: false })
    },
    loadScripts: async (reload = false) => {
      if (get().loading || (get().loaded && !reload)) return
      set({ loading: true, error: null })
      try {
        const result = await call(
          reload ? 'automation.scripts_reload' : 'automation.scripts_list',
          {},
        )
        const selected = get().selectedScript
        set({ ...result, loaded: true })
        if (
          reload &&
          selected &&
          result.scripts.some((script) => script.name === selected)
        )
          await get().openScript(selected)
      } catch (error) {
        set({ error: errorText(error) })
      } finally {
        set({ loading: false })
      }
    },
    openScript: async (name) => {
      if (get().running) return
      const request = ++sourceRequest
      set({ open: true, reading: true, error: null })
      try {
        const result = await call('automation.script_read', { name })
        if (request !== sourceRequest) return
        set({
          source: result.source,
          language: result.language,
          selectedScript: result.name,
        })
      } catch (error) {
        if (request === sourceRequest) set({ error: errorText(error) })
      } finally {
        if (request === sourceRequest) set({ reading: false })
      }
    },
    run: async () => {
      const { source, language, running, reading } = get()
      if (running || reading || !source.trim()) return
      const context = currentMacroContext()
      set({
        running: true,
        status: 'running',
        output: [],
        error: null,
        context,
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
            })
          const result =
            context.doc === null
              ? await invoke()
              : await withSettledDocument(context.doc, invoke)
          for (const text of result.output) append({ level: 'log', text })
          if (result.error) append({ level: 'error', text: result.error })
          set({ status: result.error ? 'failed' : 'completed' })
        } else {
          const run = runJavaScript(source, context, {
            call: macroCall,
            onOutput: append,
          })
          cancelRun = run.stop
          const result = await run.done
          if (result.error) append({ level: 'error', text: result.error })
          set({ status: result.status })
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
      if (!get().running) set({ output: [], status: 'idle', error: null })
    },
  }
})

/** Opens the console and discovers local scripts without executing any source. */
export function openMacroConsole(): void {
  useAutomationStore.setState({ open: true })
  void useAutomationStore.getState().loadScripts()
}
