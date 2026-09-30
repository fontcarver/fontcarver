import { create } from 'zustand'

import {
  bundledPlugin,
  curatedIndex,
  defaultSettings,
  normalizeSetting,
  pluginEntry,
} from './registry'

/** Versioned local preference key; the record contains no executable code. */
export const PLUGIN_STORAGE_KEY = 'fontcarver.plugins.v2'

type PluginStorage = Pick<Storage, 'getItem' | 'setItem'>

/** A local installation always refers to the application's bundled version. */
export interface InstalledPlugin {
  enabled: boolean
  version: string
  /** Persisted settings, already clamped to the index's declared ranges. */
  settings: Record<string, number>
}

/** Explicit lifecycle preferences for the curated, bundled plugin index. */
export interface PluginState {
  installed: Record<string, InstalledPlugin>
  install: (id: string) => void
  enable: (id: string) => void
  disable: (id: string) => void
  remove: (id: string) => void
  /** Re-reads the persisted record and re-validates it against the index. */
  reload: () => void
  setSetting: (id: string, key: string, value: number) => void
}

function browserStorage(): PluginStorage | null {
  try {
    return globalThis.localStorage ?? null
  } catch {
    return null
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/** Validates one persisted settings record against the plugin's contract. */
function loadSettings(id: string, stored: unknown): Record<string, number> {
  const entry = pluginEntry(id)
  if (!entry) return {}
  const source = isRecord(stored) ? stored : {}
  return Object.fromEntries(
    entry.settings.map((setting) => [
      setting.key,
      normalizeSetting(setting, source[setting.key]),
    ]),
  )
}

function loadInstalled(
  storage: PluginStorage | null,
): PluginState['installed'] {
  const installed: PluginState['installed'] = {}
  try {
    const serialized = storage?.getItem(PLUGIN_STORAGE_KEY)
    if (!serialized) return installed
    const saved: unknown = JSON.parse(serialized)
    if (!isRecord(saved)) return installed
    for (const entry of curatedIndex) {
      const stored = saved[entry.id]
      if (isRecord(stored) && typeof stored.enabled === 'boolean')
        installed[entry.id] = {
          enabled: stored.enabled,
          // A stored version never overrides the shipped implementation.
          version: entry.version,
          settings: loadSettings(entry.id, stored.settings),
        }
    }
  } catch {
    // Unavailable storage and corrupt data leave a usable, empty registry.
  }
  return installed
}

/** Creates an isolated lifecycle store; null opts out of persistence. */
export function createPluginStore(
  storage: PluginStorage | null = browserStorage(),
) {
  return create<PluginState>((set, get) => {
    const update = (installed: PluginState['installed']): void => {
      set({ installed })
      try {
        storage?.setItem(PLUGIN_STORAGE_KEY, JSON.stringify(installed))
      } catch {
        // Lifecycle changes remain effective when browser storage is blocked/full.
      }
    }

    const setEnabled = (id: string, enabled: boolean): void => {
      if (!bundledPlugin(id)) return
      const installed = get().installed
      const entry = installed[id]
      if (!entry || entry.enabled === enabled) return
      update({ ...installed, [id]: { ...entry, enabled } })
    }

    return {
      installed: loadInstalled(storage),
      install: (id) => {
        const plugin = bundledPlugin(id)
        const installed = get().installed
        if (!plugin || installed[id]) return
        update({
          ...installed,
          [id]: {
            enabled: false,
            version: plugin.manifest.version,
            settings: defaultSettings(id),
          },
        })
      },
      enable: (id) => setEnabled(id, true),
      disable: (id) => setEnabled(id, false),
      remove: (id) => {
        if (!bundledPlugin(id) || !get().installed[id]) return
        const installed = { ...get().installed }
        delete installed[id]
        update(installed)
      },
      reload: () => {
        // Bundled code is compiled into the application, so reload re-reads the
        // curated index and the persisted record rather than fetching anything.
        set({ installed: loadInstalled(storage) })
      },
      setSetting: (id, key, value) => {
        const installed = get().installed
        const entry = installed[id]
        const setting = pluginEntry(id)?.settings.find(
          (candidate) => candidate.key === key,
        )
        if (!entry || !setting) return
        const next = normalizeSetting(setting, value)
        if (entry.settings[key] === next) return
        update({
          ...installed,
          [id]: { ...entry, settings: { ...entry.settings, [key]: next } },
        })
      },
    }
  })
}

/** Shared plugin preferences for management and the active editor overlays. */
export const usePluginStore = createPluginStore()

/** Settings of an installed plugin, or its declared defaults. */
export function pluginSettings(id: string): Readonly<Record<string, number>> {
  return (
    usePluginStore.getState().installed[id]?.settings ?? defaultSettings(id)
  )
}
