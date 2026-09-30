import { afterEach, describe, expect, it, vi } from 'vitest'

import { createPluginStore, PLUGIN_STORAGE_KEY } from './pluginStore'
import { boundsReporter, translateFilter } from './registry'

const boundsId = boundsReporter.manifest.id
const translateId = translateFilter.manifest.id

function memoryStorage(initial: string | null = null) {
  let saved = initial
  return {
    getItem: vi.fn(() => saved),
    setItem: vi.fn((_key: string, value: string) => {
      saved = value
    }),
  }
}

afterEach(() => vi.unstubAllGlobals())

describe('plugin lifecycle', () => {
  it('starts with nothing installed and requires a separate enable action', () => {
    const storage = memoryStorage()
    const store = createPluginStore(storage)
    expect(store.getState().installed).toEqual({})
    expect(storage.setItem).not.toHaveBeenCalled()

    store.getState().enable(boundsId)
    expect(store.getState().installed).toEqual({})
    store.getState().install(boundsId)
    expect(store.getState().installed[boundsId]).toEqual({
      enabled: false,
      version: boundsReporter.manifest.version,
    })
    store.getState().enable(boundsId)
    expect(store.getState().installed[boundsId]?.enabled).toBe(true)
    store.getState().disable(boundsId)
    expect(store.getState().installed[boundsId]?.enabled).toBe(false)
  })

  it('persists enabled and disabled installations and removal across restarts', () => {
    const storage = memoryStorage()
    const store = createPluginStore(storage)
    store.getState().install(boundsId)
    store.getState().install(translateId)
    store.getState().enable(boundsId)

    const restarted = createPluginStore(storage)
    expect(restarted.getState().installed).toEqual({
      [boundsId]: { enabled: true, version: boundsReporter.manifest.version },
      [translateId]: {
        enabled: false,
        version: translateFilter.manifest.version,
      },
    })
    restarted.getState().disable(boundsId)
    expect(
      createPluginStore(storage).getState().installed[boundsId]?.enabled,
    ).toBe(false)
    restarted.getState().remove(boundsId)
    const removed = createPluginStore(storage)
    expect(removed.getState().installed[boundsId]).toBeUndefined()
    expect(removed.getState().installed[translateId]?.enabled).toBe(false)
    removed.getState().enable(boundsId)
    expect(removed.getState().installed[boundsId]).toBeUndefined()
    removed.getState().install(boundsId)
    expect(removed.getState().installed[boundsId]?.enabled).toBe(false)
    expect(storage.setItem).toHaveBeenLastCalledWith(
      PLUGIN_STORAGE_KEY,
      expect.any(String),
    )
  })

  it('makes repeated lifecycle actions harmless without resetting an enabled install', () => {
    const storage = memoryStorage()
    const store = createPluginStore(storage)
    store.getState().install(boundsId)
    store.getState().enable(boundsId)
    storage.setItem.mockClear()
    store.getState().install(boundsId)
    store.getState().enable(boundsId)
    store.getState().disable(translateId)
    store.getState().remove(translateId)
    expect(store.getState().installed[boundsId]?.enabled).toBe(true)
    expect(storage.setItem).not.toHaveBeenCalled()
  })

  it.each([
    'unknown',
    'https://example.com/plugin.js',
    '__proto__',
    'constructor',
  ])('ignores untrusted plugin id %s for every action', (id) => {
    const storage = memoryStorage()
    const store = createPluginStore(storage)
    store.getState().install(id)
    store.getState().enable(id)
    store.getState().disable(id)
    store.getState().remove(id)
    expect(store.getState().installed).toEqual({})
    expect(storage.setItem).not.toHaveBeenCalled()
  })
})

describe('plugin persistence validation', () => {
  it('hydrates only installed bundled ids with the current bundled version', () => {
    const storage = memoryStorage(
      JSON.stringify({
        [boundsId]: { enabled: true, version: '0.1.0' },
        'remote.plugin': { enabled: true, version: '1.0.0' },
      }),
    )
    const store = createPluginStore(storage)
    expect(store.getState().installed).toEqual({
      [boundsId]: { enabled: true, version: boundsReporter.manifest.version },
    })
    expect(storage.getItem).toHaveBeenCalledWith(PLUGIN_STORAGE_KEY)
    expect(storage.setItem).not.toHaveBeenCalled()
  })

  it.each(['broken JSON', 'null', '[]', 'true', '42', '"text"'])(
    'ignores invalid saved state %s',
    (saved) => {
      expect(
        createPluginStore(memoryStorage(saved)).getState().installed,
      ).toEqual({})
    },
  )

  it.each([
    null,
    [],
    true,
    {},
    { enabled: true },
    { enabled: 'true', version: '1.0.0' },
    { enabled: true, version: 1 },
    { enabled: true, version: '' },
  ])(
    'rejects a malformed installation %j while keeping valid peers',
    (entry) => {
      const storage = memoryStorage(
        JSON.stringify({
          [boundsId]: entry,
          [translateId]: {
            enabled: false,
            version: translateFilter.manifest.version,
          },
        }),
      )
      expect(createPluginStore(storage).getState().installed).toEqual({
        [translateId]: {
          enabled: false,
          version: translateFilter.manifest.version,
        },
      })
    },
  )

  it('keeps the lifecycle usable when storage access fails', () => {
    const storage = {
      getItem: () => {
        throw new Error('blocked')
      },
      setItem: () => {
        throw new Error('full')
      },
    }
    const store = createPluginStore(storage)
    store.getState().install(boundsId)
    store.getState().enable(boundsId)
    expect(store.getState().installed[boundsId]?.enabled).toBe(true)
    store.getState().remove(boundsId)
    expect(store.getState().installed).toEqual({})
  })

  it('allows an explicitly nonpersistent store and an unavailable browser store', () => {
    vi.stubGlobal('localStorage', undefined)
    for (const store of [createPluginStore(null), createPluginStore()]) {
      store.getState().install(boundsId)
      expect(store.getState().installed[boundsId]?.enabled).toBe(false)
    }
  })
})
