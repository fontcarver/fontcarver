import { beforeEach, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport } from '../test/mockTransport'
import { initialAutomationState, useAutomationStore } from './automationStore'
import { runJavaScript } from './runtime'

vi.mock('./runtime', () => ({ runJavaScript: vi.fn() }))

beforeEach(() => {
  vi.clearAllMocks()
  useUiStore.setState(initialUiState())
  useAutomationStore.setState(initialAutomationState())
})

it('discovers and loads local scripts without executing on reload or selection', async () => {
  const transport = createMockTransport({
    'automation.scripts_list': () => ({
      directory: '/config/scripts',
      scripts: [{ name: 'report.js', language: 'javascript', bytes: 20 }],
      warnings: [],
    }),
    'automation.scripts_reload': () => ({
      directory: '/config/scripts',
      scripts: [],
      warnings: ['Skipped large file'],
    }),
    'automation.script_read': () => ({
      name: 'report.js',
      language: 'javascript',
      source: 'console.log(1)',
    }),
  })
  setTransport(transport)
  await useAutomationStore.getState().loadScripts()
  await useAutomationStore.getState().loadScripts()
  await useAutomationStore.getState().openScript('report.js')
  await useAutomationStore.getState().loadScripts(true)
  expect(transport.calls.map(({ method }) => method)).toEqual([
    'automation.scripts_list',
    'automation.script_read',
    'automation.scripts_reload',
  ])
  expect(useAutomationStore.getState()).toMatchObject({
    open: true,
    source: 'console.log(1)',
    warnings: ['Skipped large file'],
  })
  expect(runJavaScript).not.toHaveBeenCalled()
})

it('ignores stale source reads when the user starts another selection', async () => {
  let resolveFirst!: (value: {
    name: string
    language: 'javascript'
    source: string
  }) => void
  setTransport(
    createMockTransport({
      'automation.script_read': ({ name }) =>
        name === 'first.js'
          ? new Promise((resolve) => {
              resolveFirst = resolve
            })
          : { name, language: 'javascript', source: 'second' },
    }),
  )
  const first = useAutomationStore.getState().openScript('first.js')
  await useAutomationStore.getState().openScript('second.js')
  resolveFirst({ name: 'first.js', language: 'javascript', source: 'first' })
  await first
  expect(useAutomationStore.getState().source).toBe('second')
})

it('runs Python only on Run and surfaces output and interpreter errors', async () => {
  const transport = createMockTransport({
    'automation.python_run': () => ({
      output: ['before failure'],
      error: 'NameError: unknown_name',
      elapsed_ms: 8,
      rpc_calls: 0,
    }),
  })
  setTransport(transport)
  useAutomationStore.getState().setLanguage('python')
  useAutomationStore.getState().setSource('print(unknown_name)')
  expect(transport.calls).toHaveLength(0)
  await useAutomationStore.getState().run()
  expect(transport.calls[0]).toMatchObject({
    method: 'automation.python_run',
    params: {
      source: 'print(unknown_name)',
      doc: null,
      master: null,
      glyph: null,
      timeout_ms: 10000,
    },
  })
  expect(useAutomationStore.getState()).toMatchObject({
    status: 'failed',
    running: false,
    output: [
      { level: 'log', text: 'before failure' },
      { level: 'error', text: 'NameError: unknown_name' },
    ],
  })
})

it('keeps Run disabled and Stop pending until dispatched changes have drained', async () => {
  let finish!: (value: { status: 'stopped' }) => void
  const stop = vi.fn()
  vi.mocked(runJavaScript).mockReturnValue({
    stop,
    done: new Promise((resolve) => {
      finish = resolve
    }),
  })
  useAutomationStore.getState().setSource('await fc.call("session.list", {})')
  const running = useAutomationStore.getState().run()
  await vi.waitFor(() => expect(runJavaScript).toHaveBeenCalledOnce())
  useAutomationStore.getState().stop()
  expect(stop).toHaveBeenCalledOnce()
  expect(useAutomationStore.getState()).toMatchObject({
    running: true,
    status: 'stopping',
  })
  await useAutomationStore.getState().run()
  expect(runJavaScript).toHaveBeenCalledOnce()
  finish({ status: 'stopped' })
  await running
  expect(useAutomationStore.getState()).toMatchObject({
    running: false,
    status: 'stopped',
  })
})

it('reloads unchanged selected source after an external edit and preserves console drafts', async () => {
  let source = 'original'
  const scripts = [
    { name: 'reload.js', language: 'javascript' as const, bytes: 8 },
  ]
  setTransport(
    createMockTransport({
      'automation.scripts_reload': () => ({
        directory: '/config/scripts',
        scripts,
        warnings: [],
      }),
      'automation.script_read': () => ({
        name: 'reload.js',
        language: 'javascript',
        source,
      }),
    }),
  )
  await useAutomationStore.getState().openScript('reload.js')
  source = 'updated on disk'
  await useAutomationStore.getState().loadScripts(true)
  expect(useAutomationStore.getState().source).toBe('updated on disk')
  useAutomationStore.getState().setSource('unsaved console draft')
  source = 'updated again'
  await useAutomationStore.getState().loadScripts(true)
  expect(useAutomationStore.getState().source).toBe('unsaved console draft')
})
