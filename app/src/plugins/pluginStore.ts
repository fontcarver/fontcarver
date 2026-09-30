import { create } from 'zustand'

import { bundledPlugin, bundledPlugins } from './registry'

/** Versioned local preference key; the record contains no executable code. */
export const PLUGIN_STORAGE_KEY = 'fontcarver.plugins.v1'

type PluginStorage = Pick<Storage, 'getItem' | 'setItem'>

/** A local installation always refers to the application's bundled version. */
export interface InstalledPlugin {
  enabled: boolean
  version: string
}

/** Explicit lifecycle preferences for the curated, bundled plugin index. */
export interface PluginState {
  installed: Record<string, InstalledPlugin>
  install: (id: string) => void
  enable: (id: string) => void
  disable: (id: string) => void
  remove: (id: string) => void
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

function loadInstalled(
  storage: PluginStorage | null,
): PluginState['installed'] {
  const installed: PluginState['installed'] = {}
  try {
    const serialized = storage?.getItem(PLUGIN_STORAGE_KEY)
    if (!serialized) return installed
    const saved: unknown = JSON.parse(serialized)
    if (!isRecord(saved)) return installed
    for (const { manifest } of bundledPlugins) {
      const entry = saved[manifest.id]
      if (
        isRecord(entry) &&
        typeof entry.enabled === 'boolean' &&
        typeof entry.version === 'string' &&
        entry.version.trim().length > 0
      ) {
        installed[manifest.id] = {
          enabled: entry.enabled,
          version: manifest.version,
        }
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
          [id]: { enabled: false, version: plugin.manifest.version },
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
    }
  })
}

/** Shared plugin preferences for management and the active editor overlays. */
export const usePluginStore = createPluginStore()
