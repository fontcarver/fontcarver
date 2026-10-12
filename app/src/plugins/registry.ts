import curated from './index.json'
import type {
  BundledPlugin,
  FilterPlugin,
  PluginIndexEntry,
  PluginSetting,
} from './contracts'
import { boundsReporter, metricsReporter } from './reporters'

/** Translate previews through the revision-checked, undoable style batch operation. */
export const translateFilter: FilterPlugin = {
  manifest: {
    id: 'fontcarver.translate',
    version: '1.0.0',
    kind: 'filter',
    nameKey: 'plugins.translateName',
    descriptionKey: 'plugins.translateDescription',
  },
  plan: (target, parameters, label) => {
    const { dx = 0, dy = 0 } = parameters
    if (
      ![dx, dy].every(
        (value) => Number.isFinite(value) && Math.abs(value) <= 32767,
      ) ||
      (dx === 0 && dy === 0)
    )
      throw new Error('plugins.invalidOffset')
    return {
      doc: target.doc,
      glyphs: [target.gid],
      master: target.master,
      options: {
        scale: [1, 1],
        translate: [dx, dy],
        oblique: 0,
        weight: [0, 0],
      },
      scale_advance: false,
      center: false,
      backup: false,
      label,
    }
  },
}

/** Every shipped implementation, keyed by the curated index's plugin id. */
const implementations: readonly BundledPlugin[] = [
  boundsReporter,
  metricsReporter,
  translateFilter,
]

function validSetting(value: unknown): value is PluginSetting {
  if (typeof value !== 'object' || value === null) return false
  const setting = value as Record<string, unknown>
  return (
    typeof setting.key === 'string' &&
    setting.key.length > 0 &&
    typeof setting.labelKey === 'string' &&
    (setting.kind === 'number' || setting.kind === 'toggle') &&
    ['default', 'min', 'max', 'step'].every(
      (field) => typeof setting[field] === 'number',
    ) &&
    Number(setting.min) <= Number(setting.default) &&
    Number(setting.default) <= Number(setting.max) &&
    Number(setting.step) > 0
  )
}

/**
 * Reads the bundled curated index, keeping only entries whose provenance is
 * complete and whose shipped implementation exists with the same kind and
 * version. A malformed entry is dropped rather than shown as installable.
 */
export function readCuratedIndex(
  source: unknown = curated,
): readonly PluginIndexEntry[] {
  if (typeof source !== 'object' || source === null) return []
  const record = source as Record<string, unknown>
  if (record.schema !== 1 || !Array.isArray(record.plugins)) return []
  const entries: PluginIndexEntry[] = []
  for (const candidate of record.plugins as unknown[]) {
    if (typeof candidate !== 'object' || candidate === null) continue
    const entry = candidate as Record<string, unknown>
    const implementation = implementations.find(
      (plugin) => plugin.manifest.id === entry.id,
    )
    if (
      !implementation ||
      implementation.manifest.kind !== entry.kind ||
      implementation.manifest.version !== entry.version ||
      typeof entry.author !== 'string' ||
      typeof entry.license !== 'string' ||
      typeof entry.source !== 'string' ||
      entry.origin !== 'bundled' ||
      typeof entry.nameKey !== 'string' ||
      typeof entry.descriptionKey !== 'string' ||
      !Array.isArray(entry.settings) ||
      !entry.settings.every(validSetting) ||
      entries.some((existing) => existing.id === entry.id)
    )
      continue
    entries.push({
      id: implementation.manifest.id,
      version: implementation.manifest.version,
      kind: implementation.manifest.kind,
      nameKey: entry.nameKey,
      descriptionKey: entry.descriptionKey,
      author: entry.author,
      license: entry.license,
      origin: 'bundled',
      source: entry.source,
      settings: entry.settings as PluginSetting[],
    })
  }
  return entries
}

/** The validated curated index; the notice states that nothing is downloaded. */
export const curatedIndex = readCuratedIndex()

/**
 * The index file's own local-only statement.
 *
 * Provenance metadata for reviewers and tests, not UI text: the manager shows
 * the translated `plugins.localOnly` string instead.
 */
export const curatedNotice =
  typeof (curated as { notice?: unknown }).notice === 'string'
    ? (curated as { notice: string }).notice
    : ''

/** Index entries in listing order. */
export const bundledPlugins: readonly BundledPlugin[] = curatedIndex
  .map((entry) =>
    implementations.find((plugin) => plugin.manifest.id === entry.id),
  )
  .filter((plugin): plugin is BundledPlugin => plugin !== undefined)

/** Looks up a shipped implementation; unknown persisted ids are ignored. */
export function bundledPlugin(id: string): BundledPlugin | undefined {
  return bundledPlugins.find((plugin) => plugin.manifest.id === id)
}

/** Looks up a curated index entry, with its provenance and settings contract. */
export function pluginEntry(id: string): PluginIndexEntry | undefined {
  return curatedIndex.find((entry) => entry.id === id)
}

/** Clamps a stored value into the setting's declared range and step. */
export function normalizeSetting(
  setting: PluginSetting,
  value: unknown,
): number {
  const numeric =
    typeof value === 'number' && Number.isFinite(value)
      ? value
      : setting.default
  const clamped = Math.min(setting.max, Math.max(setting.min, numeric))
  if (setting.kind === 'toggle') return clamped >= 0.5 ? 1 : 0
  // Rounding to the step can leave the range again, so clamp once more.
  const stepped = Math.round(clamped / setting.step) * setting.step
  return Math.min(setting.max, Math.max(setting.min, stepped))
}

/** The default settings record of one plugin. */
export function defaultSettings(id: string): Record<string, number> {
  const entry = pluginEntry(id)
  if (!entry) return {}
  return Object.fromEntries(
    entry.settings.map((setting) => [setting.key, setting.default]),
  )
}
