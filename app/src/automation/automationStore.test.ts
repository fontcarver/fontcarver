import { beforeEach, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport } from '../test/mockTransport'
import {
  initialAutomationState,
  TRANSACTION_LIFETIME_MS,
  useAutomationStore,
} from './automationStore'
import { runJavaScript } from './runtime'

vi.mock('./runtime', () => ({ runJavaScript: vi.fn() }))

const FOLDERS = [
  { kind: 'preferences' as const, path: '/config/scripts', exists: true },
]

const settings = { scripts_directory: null }

beforeEach(() => {
  vi.clearAllMocks()
  useUiStore.setState(initialUiState())
  useAutomationStore.setState(initialAutomationState())
})

it('discovers and loads local scripts without executing on reload or selection', async () => {
  const transport = createMockTransport({
    'automation.scripts_list': () => ({
      folders: FOLDERS,
      scripts: [
        {
          name: 'report.js',
          folder: 'preferences',
          language: 'javascript',
          bytes: 20,
        },
      ],
      warnings: [],
      fingerprint: 'aaa',
    }),
    'automation.scripts_reload': () => ({
      folders: FOLDERS,
      scripts: [],
      warnings: ['Skipped large file'],
      fingerprint: 'bbb',
    }),
    'automation.script_read': () => ({
      name: 'report.js',
      folder: 'preferences',
      language: 'javascript',
      source: 'console.log(1)',
    }),
    'automation.settings_get': () => settings,
  })
  setTransport(transport)
  await useAutomationStore.getState().loadScripts()
  await useAutomationStore.getState().loadScripts()
  await useAutomationStore.getState().openScript('report.js', 'preferences')
  await useAutomationStore.getState().loadScripts(true)
  expect(
    transport.calls
      .map(({ method }) => method)
      .filter((method) => method !== 'automation.settings_get'),
  ).toEqual([
    'automation.scripts_list',
    'automation.script_read',
    'automation.scripts_reload',
  ])
  expect(useAutomationStore.getState()).toMatchObject({
    open: true,
    source: 'console.log(1)',
    warnings: ['Skipped large file'],
    folders: FOLDERS,
    fingerprint: 'bbb',
  })
  expect(runJavaScript).not.toHaveBeenCalled()
})

it('ignores stale source reads when the user starts another selection', async () => {
  let resolveFirst!: (value: {
    name: string
    folder: 'preferences'
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
          : {
              name,
              folder: 'preferences',
              language: 'javascript',
              source: 'second',
            },
    }),
  )
  const first = useAutomationStore
    .getState()
    .openScript('first.js', 'preferences')
  await useAutomationStore.getState().openScript('second.js', 'preferences')
  resolveFirst({
    name: 'first.js',
    folder: 'preferences',
    language: 'javascript',
    source: 'first',
  })
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
      undo_label: null,
    }),
  })
  setTransport(transport)
  useAutomationStore.getState().setLanguage('python')
  useAutomationStore.getState().setSource('print(unknown_name)')
  expect(transport.calls).toHaveLength(0)
  await useAutomationStore.getState().run('Script: draft')
  expect(transport.calls[0]).toMatchObject({
    method: 'automation.python_run',
    params: {
      source: 'print(unknown_name)',
      doc: null,
      master: null,
      glyph: null,
      timeout_ms: 10000,
      label: 'Script: draft',
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
  const running = useAutomationStore.getState().run('Script: draft')
  await vi.waitFor(() => expect(runJavaScript).toHaveBeenCalledOnce())
  useAutomationStore.getState().stop()
  expect(stop).toHaveBeenCalledOnce()
  expect(useAutomationStore.getState()).toMatchObject({
    running: true,
    status: 'stopping',
  })
  await useAutomationStore.getState().run('Script: draft')
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
    {
      name: 'reload.js',
      folder: 'preferences' as const,
      language: 'javascript' as const,
      bytes: 8,
    },
  ]
  setTransport(
    createMockTransport({
      'automation.scripts_reload': () => ({
        folders: FOLDERS,
        scripts,
        warnings: [],
        fingerprint: 'aaa',
      }),
      'automation.script_read': () => ({
        name: 'reload.js',
        folder: 'preferences',
        language: 'javascript',
        source,
      }),
      'automation.settings_get': () => settings,
    }),
  )
  await useAutomationStore.getState().openScript('reload.js', 'preferences')
  source = 'updated on disk'
  await useAutomationStore.getState().loadScripts(true)
  expect(useAutomationStore.getState().source).toBe('updated on disk')
  useAutomationStore.getState().setSource('unsaved console draft')
  source = 'updated again'
  await useAutomationStore.getState().loadScripts(true)
  expect(useAutomationStore.getState().source).toBe('unsaved console draft')
})

it('wraps a JavaScript run against a document in one history transaction', async () => {
  vi.mocked(runJavaScript).mockReturnValue({
    stop: vi.fn(),
    done: Promise.resolve({ status: 'completed' as const }),
  })
  const transport = createMockTransport({
    'automation.transaction_begin': () => ({ doc: 3, token: 9 }),
    'automation.transaction_end': () => ({
      ended: true,
      undo_label: 'Script: batch.js',
    }),
  })
  setTransport(transport)
  vi.spyOn(
    await import('./macroBridge'),
    'currentMacroContext',
  ).mockReturnValue({ doc: 3, master: null, glyph: null })

  useAutomationStore
    .getState()
    .setSource('await fc.call("doc.summary", {doc: 3})')
  await useAutomationStore.getState().run('Script: batch.js')

  expect(transport.calls.map(({ method }) => method)).toEqual([
    'automation.transaction_begin',
    'automation.transaction_end',
  ])
  expect(transport.calls[0].params).toMatchObject({
    doc: 3,
    label: 'Script: batch.js',
    timeout_ms: TRANSACTION_LIFETIME_MS,
  })
  expect(transport.calls[1].params).toMatchObject({ doc: 3, token: 9 })
  expect(useAutomationStore.getState()).toMatchObject({
    status: 'completed',
    historyLabel: 'Script: batch.js',
  })
})

it('closes the transaction when the run fails', async () => {
  vi.mocked(runJavaScript).mockImplementation(() => {
    throw new Error('worker unavailable')
  })
  const transport = createMockTransport({
    'automation.transaction_begin': () => ({ doc: 4, token: 1 }),
    'automation.transaction_end': () => ({ ended: true, undo_label: null }),
  })
  setTransport(transport)
  vi.spyOn(
    await import('./macroBridge'),
    'currentMacroContext',
  ).mockReturnValue({ doc: 4, master: null, glyph: null })

  useAutomationStore.getState().setSource('fc.call("doc.summary", {doc: 4})')
  await useAutomationStore.getState().run('Script: draft')

  expect(transport.calls.map(({ method }) => method)).toEqual([
    'automation.transaction_begin',
    'automation.transaction_end',
  ])
  expect(useAutomationStore.getState().status).toBe('failed')
})

it('refuses to run a macro whose call names an unknown method', async () => {
  setTransport(createMockTransport({}))
  useAutomationStore
    .getState()
    .setSource('await fc.call("doc.nonexistent", { doc: 1 })')
  expect(useAutomationStore.getState().problems).toEqual([
    {
      severity: 'error',
      line: 1,
      key: 'automation.check.unknownMethod',
      values: { method: 'doc.nonexistent' },
    },
  ])
  await useAutomationStore.getState().run('Script: draft')
  expect(runJavaScript).not.toHaveBeenCalled()
  expect(useAutomationStore.getState().status).toBe('failed')
})

it('stores and validates the extra scripts folder, then rediscovers', async () => {
  const transport = createMockTransport({
    'automation.settings_set': ({ scripts_directory }) => ({
      scripts_directory,
    }),
    'automation.settings_get': () => ({ scripts_directory: '/team/scripts' }),
    'automation.scripts_reload': () => ({
      folders: [
        { kind: 'configured', path: '/team/scripts', exists: true },
        ...FOLDERS,
      ],
      scripts: [],
      warnings: [],
      fingerprint: 'ccc',
    }),
  })
  setTransport(transport)
  await useAutomationStore.getState().setConfiguredDirectory(' /team/scripts ')
  expect(transport.calls[0]).toMatchObject({
    method: 'automation.settings_set',
    params: { scripts_directory: '/team/scripts' },
  })
  expect(useAutomationStore.getState()).toMatchObject({
    configuredDirectory: '/team/scripts',
    fingerprint: 'ccc',
  })
  expect(useAutomationStore.getState().folders).toHaveLength(2)
})

it('reloads on change only when the folder fingerprint moves', async () => {
  let fingerprint = 'one'
  const transport = createMockTransport({
    'automation.scripts_list': () => ({
      folders: FOLDERS,
      scripts: [],
      warnings: [],
      fingerprint,
    }),
    'automation.scripts_reload': () => ({
      folders: FOLDERS,
      scripts: [],
      warnings: [],
      fingerprint,
    }),
    'automation.settings_get': () => settings,
  })
  setTransport(transport)
  useAutomationStore.setState({ open: true })
  await useAutomationStore.getState().loadScripts()
  const before = transport.calls.length
  await useAutomationStore.getState().checkFolders()
  expect(useAutomationStore.getState().changedOnDisk).toBe(false)
  expect(transport.calls.length).toBe(before + 1)

  fingerprint = 'two'
  await useAutomationStore.getState().checkFolders()
  expect(useAutomationStore.getState()).toMatchObject({
    changedOnDisk: true,
    fingerprint: 'two',
  })
})

it('keeps a rejected extra folder in the console and reports the error', async () => {
  const transport = createMockTransport({
    'automation.settings_set': () => {
      throw new Error('the scripts folder must be an absolute path')
    },
  })
  setTransport(transport)
  const saved = await useAutomationStore
    .getState()
    .setConfiguredDirectory('relative/scripts')
  expect(saved).toBe(false)
  expect(useAutomationStore.getState()).toMatchObject({
    configuredDirectory: '',
    error: 'the scripts folder must be an absolute path',
  })
})

it('reports a failed transaction close without losing the run result', async () => {
  vi.mocked(runJavaScript).mockReturnValue({
    stop: vi.fn(),
    done: Promise.resolve({ status: 'completed' as const }),
  })
  const transport = createMockTransport({
    'automation.transaction_begin': () => ({ doc: 5, token: 2 }),
    'automation.transaction_end': () => {
      throw new Error('transport closed')
    },
  })
  setTransport(transport)
  vi.spyOn(
    await import('./macroBridge'),
    'currentMacroContext',
  ).mockReturnValue({ doc: 5, master: null, glyph: null })

  useAutomationStore
    .getState()
    .setSource('await fc.call("doc.summary", {doc: 5})')
  await useAutomationStore.getState().run('Script: batch.js')

  // The run still reports its own outcome; the kernel reaps the transaction on
  // its deadline, so the failure is a warning rather than a lost result.
  expect(useAutomationStore.getState()).toMatchObject({
    status: 'completed',
    running: false,
    historyLabel: null,
  })
  expect(useAutomationStore.getState().output).toContainEqual({
    level: 'warn',
    text: 'transport closed',
  })
})
