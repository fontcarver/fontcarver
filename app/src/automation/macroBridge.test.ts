import { beforeEach, expect, it } from 'vitest'

import { setTransport } from '../api'
import { holdDocumentEdits } from '../editor/editSession'
import { initialDocumentState, useDocumentStore } from '../stores/documentStore'
import { initialEditorState, useEditorStore } from '../stores/editorStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { createMockTransport, makeSummary } from '../test/mockTransport'
import { currentMacroContext, macroCall } from './macroBridge'

beforeEach(() => {
  useDocumentStore.setState(initialDocumentState())
  useEditorStore.setState(initialEditorState())
  useUiStore.setState(initialUiState())
})

it('holds an RPC until existing document edits settle', async () => {
  let release!: () => void
  const pending = new Promise<void>((resolve) => {
    release = resolve
  })
  void holdDocumentEdits(1, pending)
  const transport = createMockTransport({ 'doc.summary': () => makeSummary() })
  setTransport(transport)
  const operation = macroCall('doc.summary', { doc: 1 })
  expect(transport.calls).toHaveLength(0)
  release()
  await operation
  expect(transport.calls).toEqual([
    { method: 'doc.summary', params: { doc: 1 } },
  ])
})

it('has no document context when the workspace has no active tab', () => {
  expect(currentMacroContext()).toEqual({
    doc: null,
    master: null,
    glyph: null,
  })
})

it('cancels an RPC waiting behind an editor barrier before dispatching a mutation', async () => {
  let release!: () => void
  void holdDocumentEdits(
    1,
    new Promise<void>((resolve) => {
      release = resolve
    }),
  )
  const transport = createMockTransport({ 'doc.summary': () => makeSummary() })
  setTransport(transport)
  const abort = new AbortController()
  const operation = macroCall('doc.summary', { doc: 1 }, abort.signal)
  abort.abort()
  release()
  await expect(operation).rejects.toThrow('Macro execution has stopped')
  expect(transport.calls).toHaveLength(0)
})

it('captures the active font selection and master', () => {
  useUiStore.setState({
    tabs: [{ id: 'font-2', kind: 'font', doc: 2 }],
    activeTab: 'font-2',
  })
  useDocumentStore.getState().ensure(2)
  useDocumentStore.getState().setMaster(2, 'bold')
  useDocumentStore.getState().setFocus(2, 17)
  expect(currentMacroContext()).toEqual({ doc: 2, master: 'bold', glyph: 17 })
})

it('settles source-document edits before a cross-document RPC and deduplicates the same document', async () => {
  let release!: () => void
  void holdDocumentEdits(
    2,
    new Promise<void>((resolve) => {
      release = resolve
    }),
  )
  const transport = createMockTransport({ 'doc.summary': () => makeSummary() })
  setTransport(transport)
  // Structural params exercise the shared bridge before optional cross-document
  // method bindings are regenerated. Unknown API fields are forward-compatible.
  const params = { doc: 1, source_doc: 2 }
  const operation = macroCall('doc.summary', params)
  await new Promise((resolve) => setTimeout(resolve, 0))
  expect(transport.calls).toHaveLength(0)
  release()
  await operation
  expect(transport.calls).toHaveLength(1)
  await macroCall('doc.summary', { doc: 1, source_doc: 1 } as typeof params)
  expect(transport.calls).toHaveLength(2)
})

it('rejects raw gesture transactions before any backend call', async () => {
  const transport = createMockTransport()
  setTransport(transport)
  await expect(
    macroCall('doc.edit_begin', { doc: 1, label: 'orphaned edit' }),
  ).rejects.toThrow('raw gesture transactions')
  expect(transport.calls).toHaveLength(0)
})
