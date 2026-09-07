import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import { initialTextState } from '../views/text/textState'
import { i18n } from '../i18n'
import { initialDocumentState, useDocumentStore } from '../stores/documentStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { registerBuiltinCommands } from './builtinCommands'
import { holdDocumentEdits } from '../editor/editSession'
import {
  getCommand,
  isCommandEnabled,
  listCommands,
  resetCommands,
  runCommand,
  setCommandErrorReporter,
} from './commands'
import { commandForEvent } from './shortcuts'

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
  it('reserves kerning arrows on a text canvas and keeps tab cycling elsewhere', () => {
    openDocument()
    const id = useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 3 })
    useUiStore
      .getState()
      .updateTab(id, { textState: initialTextState('regular', 'A') })
    const canvas = document.createElement('canvas')
    canvas.tabIndex = 0
    document.body.append(canvas)
    canvas.focus()
    expect(isCommandEnabled('window.nextTab')).toBe(false)
    expect(isCommandEnabled('text.kerning.ArrowRight')).toBe(false)
    canvas.remove()
    expect(isCommandEnabled('window.nextTab')).toBe(true)
  })

  it('gives every command a title that the catalogue resolves', () => {
    // A title key that i18next cannot resolve renders as the raw key in the
    // menus, the palette and the shortcut help. Nesting matters: a flat
    // `cmd.edit.delete` key is shadowed by the `cmd.edit` object.
    const unresolved = listCommands()
      .map((command) => command.title)
      .filter((title) => i18n.t(title) === title)

    expect(unresolved).toEqual([])
  })

  it('resolves the reason every disabled command gives', () => {
    // Menus show the reason under the greyed title, so an unresolved key is
    // just as visible there.
    const unresolved = listCommands()
      .map((command) => command.disabledReason)
      .filter(
        (reason): reason is string =>
          reason !== undefined && i18n.t(reason) === reason,
      )

    expect(unresolved).toEqual([])
  })

  it('keeps the Font and Edit tab select-all commands apart', () => {
    // Both are bound to Mod+A and both live in the Edit menu; sharing an id
    // would let the later registration silently replace the earlier one.
    expect(getCommand('edit.selectAll')?.title).toBe('cmd.edit.selectAll')
    expect(getCommand('edit.selectAllPoints')?.title).toBe(
      'cmd.edit.selectAllPoints',
    )
  })

  it('routes Mod+A to the command of the active tab kind', () => {
    const event = new KeyboardEvent('keydown', { key: 'a', ctrlKey: true })
    openDocument()
    expect(commandForEvent(event, false)).toBe('edit.selectAll')

    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 7 })
    expect(commandForEvent(event, false)).toBe('edit.selectAllPoints')
  })

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

  it('waits for this document handover before running history', async () => {
    let release = () => undefined as void
    const closing = new Promise<void>((resolve) => {
      release = resolve
    })
    holdDocumentEdits(1, closing)
    const transport = createMockTransport({
      'doc.undo': () => null,
      'session.list': () => [makeSummary({ undo_label: null })],
    })
    setTransport(transport)
    openDocument(makeSummary({ undo_label: 'Move nodes' }))

    expect(runCommand('edit.undo')).toBe(true)
    await Promise.resolve()
    expect(transport.calls).toEqual([])

    release()
    await vi.waitFor(() =>
      expect(transport.calls.map((entry) => entry.method)).toContain(
        'doc.undo',
      ),
    )
  })

  it('asks before closing a dirty document rather than discarding it', async () => {
    setTransport(
      createMockTransport({
        'doc.summary': () => makeSummary({ dirty: true }),
        'session.close': () => null,
      }),
    )
    openDocument(makeSummary({ dirty: true }))

    runCommand('file.close')

    await vi.waitFor(() =>
      expect(useUiStore.getState().closePrompt).toEqual({ doc: 1 }),
    )
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
