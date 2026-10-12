import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  createPluginStore,
  PLUGIN_STORAGE_KEY,
  pluginSettings,
  usePluginStore,
} from './pluginStore'
import { boundsReporter, metricsReporter } from './reporters'
import { defaultSettings, translateFilter } from './registry'

const boundsId = boundsReporter.manifest.id
const metricsId = metricsReporter.manifest.id
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

const boundsEntry = (enabled: boolean) => ({
  enabled,
  version: boundsReporter.manifest.version,
  settings: defaultSettings(boundsId),
})

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
    expect(store.getState().installed[boundsId]).toEqual(boundsEntry(false))
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
      [boundsId]: boundsEntry(true),
      [translateId]: {
        enabled: false,
        version: translateFilter.manifest.version,
        settings: defaultSettings(translateId),
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
    store.getState().setSetting(id, 'dx', 5)
    expect(store.getState().installed).toEqual({})
    expect(storage.setItem).not.toHaveBeenCalled()
  })
})

describe('plugin settings', () => {
  it('persists, clamps and rounds declared settings', () => {
    const storage = memoryStorage()
    const store = createPluginStore(storage)
    store.getState().install(translateId)
    expect(store.getState().installed[translateId]?.settings).toEqual({
      dx: 20,
      dy: 0,
    })

    store.getState().setSetting(translateId, 'dx', 40.4)
    store.getState().setSetting(translateId, 'dy', -100000)
    expect(store.getState().installed[translateId]?.settings).toEqual({
      dx: 40,
      dy: -32767,
    })
    expect(
      createPluginStore(storage).getState().installed[translateId]?.settings,
    ).toEqual({ dx: 40, dy: -32767 })

    // A toggle is stored as 0 or 1 regardless of the value offered.
    store.getState().install(metricsId)
    store.getState().setSetting(metricsId, 'zones', 0.2)
    expect(store.getState().installed[metricsId]?.settings).toEqual({
      zones: 0,
    })
    store.getState().setSetting(metricsId, 'zones', 7)
    expect(store.getState().installed[metricsId]?.settings).toEqual({
      zones: 1,
    })
    // An undeclared key is refused.
    store.getState().setSetting(metricsId, 'unknown', 1)
    expect(store.getState().installed[metricsId]?.settings).toEqual({
      zones: 1,
    })
  })

  it('repairs missing and non-numeric persisted settings on load', () => {
    const storage = memoryStorage(
      JSON.stringify({
        [translateId]: {
          enabled: true,
          settings: { dx: 'nope', dy: 12, extra: 4 },
        },
      }),
    )
    expect(
      createPluginStore(storage).getState().installed[translateId],
    ).toEqual({
      enabled: true,
      version: translateFilter.manifest.version,
      settings: { dx: 20, dy: 12 },
    })
  })

  it('reads the declared defaults for a plugin that is not installed', () => {
    expect(pluginSettings('unknown.plugin')).toEqual({})
    expect(pluginSettings(metricsId)).toEqual(defaultSettings(metricsId))
    expect(usePluginStore.getState().installed).toBeDefined()
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
      [boundsId]: boundsEntry(true),
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

  it.each([null, [], true, {}, { version: '1.0.0' }, { enabled: 'true' }])(
    'rejects a malformed installation %j while keeping valid peers',
    (entry) => {
      const storage = memoryStorage(
        JSON.stringify({
          [boundsId]: entry,
          [translateId]: { enabled: false },
        }),
      )
      expect(createPluginStore(storage).getState().installed).toEqual({
        [translateId]: {
          enabled: false,
          version: translateFilter.manifest.version,
          settings: defaultSettings(translateId),
        },
      })
    },
  )

  it('reloads the persisted record without fetching anything', () => {
    const storage = memoryStorage()
    const store = createPluginStore(storage)
    store.getState().install(boundsId)
    // Another window changed the shared record.
    storage.setItem(
      PLUGIN_STORAGE_KEY,
      JSON.stringify({ [metricsId]: { enabled: true } }),
    )
    store.getState().reload()
    expect(store.getState().installed).toEqual({
      [metricsId]: {
        enabled: true,
        version: metricsReporter.manifest.version,
        settings: defaultSettings(metricsId),
      },
    })
  })

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
