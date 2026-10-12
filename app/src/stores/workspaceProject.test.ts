import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { initialSessionState, useSessionStore } from './sessionStore'
import { initialUiState, useUiStore } from './uiStore'
import {
  installWorkspaceProjectSync,
  workspaceStorageOf,
} from './workspaceProject'
import { captureWorkspace } from './workspaceSnapshot'
import {
  initialWorkspaceState,
  useWorkspaceStore,
  workspaceDocumentKey,
} from './workspaceStore'

let stop: (() => void) | undefined
const project = makeSummary({ path: '/fonts/Family.fcfp' })
const key = workspaceDocumentKey(project)
const layout = captureWorkspace(project.doc, {
  ...initialUiState(),
  panels: { sidebar: false, inspector: true },
})

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
  useWorkspaceStore.setState(initialWorkspaceState())
})

afterEach(() => {
  stop?.()
  stop = undefined
  setTransport(undefined)
  vi.restoreAllMocks()
})

/** Lets the store's promise chain settle before the assertions read it. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0))

it('loads a project document stored workspaces into the store', async () => {
  const transport = createMockTransport({
    'session.workspaces': () => ({
      workspaces: [{ name: 'Two docks', layout }],
      path: '/fonts/Family.fcfp/workspaces.json',
    }),
    'session.set_workspaces': (params) => ({
      workspaces: params.workspaces,
      path: '/fonts/Family.fcfp/workspaces.json',
    }),
  })
  setTransport(transport)
  useSessionStore.setState({ docs: [project] })

  stop = installWorkspaceProjectSync()
  await settle()

  expect(workspaceStorageOf(key)).toBe('project')
  expect(
    useWorkspaceStore.getState().documents[key]?.presets.map((p) => p.name),
  ).toEqual(['Two docks'])
  // Nothing changed, so nothing is written back.
  expect(transport.calls.map((call) => call.method)).toEqual([
    'session.workspaces',
  ])
})

it('writes a newly saved workspace into the project', async () => {
  const transport = createMockTransport({
    'session.workspaces': () => ({
      workspaces: [],
      path: '/p/workspaces.json',
    }),
    'session.set_workspaces': (params) => ({
      workspaces: params.workspaces,
      path: '/p/workspaces.json',
    }),
  })
  setTransport(transport)
  useSessionStore.setState({ docs: [project] })
  stop = installWorkspaceProjectSync()
  await settle()

  useWorkspaceStore.getState().savePreset(key, 'Proofing', layout)
  await settle()

  const written = transport.calls.filter(
    (call) => call.method === 'session.set_workspaces',
  )
  expect(written).toHaveLength(1)
  expect(written[0]?.params).toMatchObject({
    doc: project.doc,
    workspaces: [{ name: 'Proofing' }],
  })

  useWorkspaceStore.getState().deletePreset(key, 'Proofing')
  await settle()
  expect(
    transport.calls.filter((call) => call.method === 'session.set_workspaces'),
  ).toHaveLength(2)
})

it('merges a workspace saved before the document had a project', async () => {
  const transport = createMockTransport({
    'session.workspaces': () => ({
      workspaces: [{ name: 'Shared', layout }],
      path: '/p/workspaces.json',
    }),
    'session.set_workspaces': (params) => ({
      workspaces: params.workspaces,
      path: '/p/workspaces.json',
    }),
  })
  setTransport(transport)
  useWorkspaceStore.getState().savePreset(key, 'Local only', layout)
  useSessionStore.setState({ docs: [project] })

  stop = installWorkspaceProjectSync()
  await settle()

  expect(
    useWorkspaceStore
      .getState()
      .documents[key]?.presets.map((preset) => preset.name)
      .sort(),
  ).toEqual(['Local only', 'Shared'])
  // The union is pushed back, so the project gains the local-only arrangement.
  const written = transport.calls.find(
    (call) => call.method === 'session.set_workspaces',
  )?.params as { workspaces: { name: string }[] } | undefined
  expect(written?.workspaces.map((entry) => entry.name).sort()).toEqual([
    'Local only',
    'Shared',
  ])
})

it('keeps a document without project storage on local records only', async () => {
  const binary = makeSummary({ doc: 2, path: '/fonts/Family.ttf' })
  const transport = createMockTransport({
    'session.workspaces': () => ({ workspaces: [], path: null }),
  })
  setTransport(transport)
  useSessionStore.setState({ docs: [binary] })
  stop = installWorkspaceProjectSync()
  await settle()

  const localKey = workspaceDocumentKey(binary)
  expect(workspaceStorageOf(localKey)).toBe('local')
  useWorkspaceStore.getState().savePreset(localKey, 'Local', layout)
  await settle()
  expect(transport.calls.map((call) => call.method)).toEqual([
    'session.workspaces',
  ])
  expect(
    useWorkspaceStore.getState().documents[localKey]?.presets,
  ).toHaveLength(1)
})

it('reports an unreadable record once and keeps the local workspaces', async () => {
  setTransport(
    createMockTransport({
      'session.workspaces': () => {
        throw new Error('workspaces.json is not a FontCarver workspace file')
      },
    }),
  )
  useWorkspaceStore.getState().savePreset(key, 'Local only', layout)
  useSessionStore.setState({ docs: [project] })

  stop = installWorkspaceProjectSync()
  await settle()

  expect(workspaceStorageOf(key)).toBe('local')
  expect(useUiStore.getState().toasts).toHaveLength(1)
  expect(useUiStore.getState().toasts[0]?.message).toContain(
    'could not be read',
  )
  expect(useWorkspaceStore.getState().documents[key]?.presets).toHaveLength(1)
})
