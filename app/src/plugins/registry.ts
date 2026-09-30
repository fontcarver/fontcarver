import type { BundledPlugin, FilterPlugin, ReporterPlugin } from './contracts'

/** Bounds uses the exact flattened outline bounds supplied by the kernel. */
export const boundsReporter: ReporterPlugin = {
  manifest: {
    id: 'fontcarver.bounds',
    version: '1.0.0',
    kind: 'reporter',
    nameKey: 'plugins.boundsName',
    descriptionKey: 'plugins.boundsDescription',
  },
  report: ({ bounds }) =>
    bounds ? [{ kind: 'bounds', bounds: { ...bounds } }] : [],
}

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

/** Curated bundled index. Installing changes local preferences, never fetches code. */
export const bundledPlugins: readonly BundledPlugin[] = [
  boundsReporter,
  translateFilter,
]

/** Looks up a shipped implementation; unknown persisted ids are ignored. */
export function bundledPlugin(id: string): BundledPlugin | undefined {
  return bundledPlugins.find((plugin) => plugin.manifest.id === id)
}
