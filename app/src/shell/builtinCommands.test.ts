import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import { initialDocumentState, useDocumentStore } from '../stores/documentStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { registerBuiltinCommands } from './builtinCommands'
import {
  isCommandEnabled,
  listCommands,
  resetCommands,
  runCommand,
  setCommandErrorReporter,
} from './commands'

function openDocument(summary = makeSummary()) {
  useSessionStore.setState({ docs: [summary] })
  useUiStore.getState().openTab({ kind: 'font', doc: summary.doc })
}

beforeEach(() => {
  localStorage.clear()
  resetCommands()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useDocumentStore.setState(initialDocumentState())
  registerBuiltinCommands()
})

afterEach(() => {
  resetCommands()
  setCommandErrorReporter(() => undefined)
  setTransport(undefined)
  vi.restoreAllMocks()
})

describe('built-in commands', () => {
  it('explains every gated command', () => {
    const gatedWithoutReason = listCommands()
      .filter((command) => command.when && !command.disabledReason)
      .map((command) => command.id)

    expect(gatedWithoutReason).toEqual([])
  })

  it('covers every menu declared by the shell', () => {
    const menus = new Set(listCommands().map((command) => command.menu))

    expect(menus).toContain('file')
    expect(menus).toContain('edit')
    expect(menus).toContain('glyph')
    expect(menus).toContain('view')
    expect(menus).toContain('window')
    expect(menus).toContain('help')
  })

  it('opens the path prompt for file.open under the HTTP transport', () => {
    setTransport(createMockTransport({}))

    expect(runCommand('file.open')).toBe(true)

    expect(useUiStore.getState().pathPrompt).toEqual({ mode: 'open' })
  })

  it('toggles panels and steps the cell size of the active document', () => {
    setTransport(createMockTransport({}))
    openDocument()

    runCommand('view.toggleSidebar')
    expect(useUiStore.getState().panels.sidebar).toBe(false)

    runCommand('view.zoomIn')
    expect(useDocumentStore.getState().byDoc[1]?.cellSize).toBe(80)
    runCommand('view.zoomOut')
    expect(useDocumentStore.getState().byDoc[1]?.cellSize).toBe(64)
  })

  it('gates document commands on there being an active document', () => {
    expect(isCommandEnabled('file.save')).toBe(false)
    expect(isCommandEnabled('file.close')).toBe(false)

    setTransport(createMockTransport({}))
    openDocument()

    expect(isCommandEnabled('file.save')).toBe(true)
    expect(isCommandEnabled('file.close')).toBe(true)
  })

  it('gates undo and redo on the kernel reporting a label', () => {
    setTransport(createMockTransport({}))
    openDocument()
    expect(isCommandEnabled('edit.undo')).toBe(false)

    useSessionStore.setState({
      docs: [makeSummary({ undo_label: 'Move nodes' })],
    })

    expect(isCommandEnabled('edit.undo')).toBe(true)
    expect(isCommandEnabled('edit.redo')).toBe(false)
  })

  it('routes an undo failure to the command error reporter', async () => {
    const reporter = vi.fn()
    setCommandErrorReporter(reporter)
    setTransport(
      createMockTransport({
        'doc.undo': () => {
          throw new Error('undo stack corrupt')
        },
      }),
    )
    openDocument(makeSummary({ undo_label: 'Move nodes' }))

    expect(runCommand('edit.undo')).toBe(true)
    await vi.waitFor(() => expect(reporter).toHaveBeenCalledOnce())

    expect(reporter.mock.calls[0]?.[0]).toMatchObject({
      message: 'undo stack corrupt',
    })
  })

  it('asks before closing a dirty document rather than discarding it', () => {
    setTransport(createMockTransport({ 'session.close': () => null }))
    openDocument(makeSummary({ dirty: true }))

    runCommand('file.close')

    expect(useUiStore.getState().closePrompt).toEqual({ doc: 1 })
  })
})

describe('glyph.open targeting', () => {
  it('opens the focus cell, not the first member of the selection', () => {
    const summary = makeSummary()
    openDocument(summary)
    const store = useDocumentStore.getState()
    store.ensure(summary.doc)
    store.setSelection(summary.doc, [5, 6, 7, 8])
    store.setFocus(summary.doc, 7)

    expect(isCommandEnabled('glyph.open')).toBe(true)
    expect(runCommand('glyph.open')).toBe(true)

    expect(useUiStore.getState().tabs.some((tab) => tab.gid === 7)).toBe(true)
    expect(useUiStore.getState().tabs.some((tab) => tab.gid === 5)).toBe(false)
  })

  it('falls back to the first selected glyph when nothing has focus', () => {
    const summary = makeSummary()
    openDocument(summary)
    const store = useDocumentStore.getState()
    store.ensure(summary.doc)
    store.setSelection(summary.doc, [11, 12])

    runCommand('glyph.open')

    expect(useUiStore.getState().tabs.some((tab) => tab.gid === 11)).toBe(true)
  })

  it('is disabled with neither a focus cell nor a selection', () => {
    openDocument()

    expect(isCommandEnabled('glyph.open')).toBe(false)
  })
})
