import { beforeEach, describe, expect, it, vi } from 'vitest'
import { FeatureMasterNavigation } from './FeatureMasterNavigation'
import { settleDocumentEdits } from '../../editor/documentPersistence'
import { useDocumentStore } from '../../stores/documentStore'
import { useUiStore } from '../../stores/uiStore'
vi.mock('../../editor/documentPersistence', () => ({
  settleDocumentEdits: vi.fn(),
}))
beforeEach(() => {
  vi.mocked(settleDocumentEdits).mockReset()
  useDocumentStore.getState().ensure(1)
  useDocumentStore.getState().setMaster(1, 'm1')
  useUiStore.setState({
    tabs: [{ id: 'features:1', doc: 1, kind: 'features' }],
    activeTab: 'features:1',
  })
})
describe('Features master navigation', () => {
  it('keeps the current master when draft persistence fails', async () => {
    vi.mocked(settleDocumentEdits).mockRejectedValueOnce(
      new Error('save failed'),
    )
    const navigation = new FeatureMasterNavigation(1)
    await expect(navigation.change('m2')).rejects.toThrow('save failed')
    expect(useDocumentStore.getState().byDoc[1]?.master).toBe('m1')
  })
  it('ignores a delayed older master choice', async () => {
    let release!: () => void
    vi.mocked(settleDocumentEdits)
      .mockImplementationOnce(
        () =>
          new Promise<void>((resolve) => {
            release = resolve
          }),
      )
      .mockResolvedValueOnce()
    const navigation = new FeatureMasterNavigation(1)
    const old = navigation.change('m2')
    await navigation.change('m3')
    release()
    await old
    expect(useDocumentStore.getState().byDoc[1]?.master).toBe('m3')
  })
  it('ignores a completion after navigating to a different tab', async () => {
    let release!: () => void
    vi.mocked(settleDocumentEdits).mockImplementationOnce(
      () =>
        new Promise<void>((resolve) => {
          release = resolve
        }),
    )
    const navigation = new FeatureMasterNavigation(1)
    const old = navigation.change('m2')
    useUiStore.setState({ activeTab: 'font:1' })
    release()
    await old
    expect(useDocumentStore.getState().byDoc[1]?.master).toBe('m1')
  })
})
