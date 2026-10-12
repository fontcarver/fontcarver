import { beforeEach, expect, it, vi } from 'vitest'
import { initialUiState, useUiStore } from '../stores/uiStore'
import {
  detachActiveTab,
  detachPanel,
  detachTab,
  rejoinDetachedWindow,
  reopenDetachedViews,
} from './windowActions'

const mocks = vi.hoisted(() => ({
  requestWindow: vi.fn(),
  rejoinWindow: vi.fn(),
  settle: vi.fn(),
}))
vi.mock('../transport/windows', () => ({
  requestWindow: mocks.requestWindow,
  rejoinWindow: mocks.rejoinWindow,
}))
vi.mock('../editor/documentPersistence', () => ({
  settleDocumentEdits: mocks.settle,
}))

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  mocks.requestWindow.mockReset().mockResolvedValue(undefined)
  mocks.rejoinWindow.mockReset().mockResolvedValue(undefined)
  mocks.settle.mockReset().mockResolvedValue(undefined)
})

it('settles a tab, transfers its text state and removes it only after opening', async () => {
  const ui = useUiStore.getState()
  ui.openTab({ kind: 'font', doc: 1 })
  const id = ui.openTab({ kind: 'glyph', doc: 1, gid: 3, label: 'AV' })
  let finish!: () => void
  mocks.requestWindow.mockReturnValue(
    new Promise<void>((resolve) => {
      finish = resolve
    }),
  )
  const pending = detachActiveTab()
  await vi.waitFor(() => expect(mocks.requestWindow).toHaveBeenCalled())
  expect(useUiStore.getState().tabs.some((tab) => tab.id === id)).toBe(true)
  finish()
  await pending
  expect(mocks.settle).toHaveBeenCalledWith(1, expect.any(Function))
  expect(mocks.requestWindow).toHaveBeenCalledWith(
    { kind: 'tab', doc: 1, tab: 'glyph', gid: 3, tabId: id },
    expect.objectContaining({ id, label: 'AV' }),
  )
  expect(useUiStore.getState().tabs.map((tab) => tab.kind)).toEqual(['font'])
})

it('preserves the source tab and panel when popup creation fails', async () => {
  const id = useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 3 })
  mocks.requestWindow.mockRejectedValue(new Error('blocked'))
  await expect(detachTab(id)).rejects.toThrow('blocked')
  await expect(detachPanel('inspector')).rejects.toThrow('blocked')
  expect(useUiStore.getState().tabs).toHaveLength(1)
  expect(useUiStore.getState().panels.inspector).toBe(true)
})

it('keeps the Overview pinned and detaches sidebar context with the active tab', async () => {
  const id = useUiStore.getState().openTab({ kind: 'font', doc: 1 })
  await detachTab(id)
  expect(mocks.requestWindow).not.toHaveBeenCalled()
  await detachPanel('sidebar')
  expect(mocks.requestWindow).toHaveBeenCalledWith(
    { kind: 'panel', doc: 1, panel: 'sidebar' },
    expect.objectContaining({ id }),
  )
  expect(useUiStore.getState().panels.sidebar).toBe(false)
})

it('settles and transfers the latest detached state before returning', async () => {
  useUiStore
    .getState()
    .openTab({ kind: 'glyph', doc: 1, gid: 3, label: 'Latest text' })
  await rejoinDetachedWindow({ kind: 'tab', doc: 1, tab: 'glyph', gid: 3 })
  expect(mocks.settle).toHaveBeenCalled()
  expect(mocks.rejoinWindow).toHaveBeenCalledWith(
    { kind: 'tab', doc: 1, tab: 'glyph', gid: 3 },
    expect.objectContaining({ label: 'Latest text' }),
  )
})

it('records every detached view so a workspace can put it back', async () => {
  const ui = useUiStore.getState()
  ui.openTab({ kind: 'font', doc: 1 })
  const glyph = ui.openTab({ kind: 'glyph', doc: 1, gid: 3 })

  await detachPanel('inspector')
  await detachTab(glyph)
  expect(useUiStore.getState().detached).toEqual([
    { kind: 'panel', doc: 1, panel: 'inspector' },
    { kind: 'tab', doc: 1, tab: 'glyph', gid: 3, tabId: glyph },
  ])
})

it('reopens a saved list and reports only the windows that were refused', async () => {
  useUiStore.getState().openTab({ kind: 'font', doc: 1 })
  mocks.requestWindow.mockImplementation((target: { panel?: string }) =>
    target.panel === 'preview'
      ? Promise.reject(new Error('blocked'))
      : Promise.resolve(undefined),
  )

  const failures = await reopenDetachedViews([
    { kind: 'panel', doc: 1, panel: 'inspector' },
    { kind: 'panel', doc: 1, panel: 'preview' },
    { kind: 'tab', doc: 1, tab: 'features', tabId: 'features:1' },
  ])

  expect(failures).toHaveLength(1)
  expect(mocks.requestWindow).toHaveBeenCalledTimes(3)
  expect(useUiStore.getState().detached).toEqual([
    { kind: 'panel', doc: 1, panel: 'inspector' },
    { kind: 'tab', doc: 1, tab: 'features', tabId: 'features:1' },
  ])
})
