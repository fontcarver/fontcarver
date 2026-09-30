import type { PluginOverlay, ReporterInput, ReporterPlugin } from './contracts'

/** Bounds uses the exact flattened outline bounds supplied by the kernel. */
export const boundsReporter: ReporterPlugin = {
  manifest: {
    id: 'fontcarver.bounds',
    version: '1.0.0',
    kind: 'reporter',
    nameKey: 'plugins.boundsName',
    descriptionKey: 'plugins.boundsDescription',
  },
  report: ({ bounds, document, settings }: ReporterInput) => {
    const overlays: PluginOverlay[] = bounds
      ? [{ kind: 'bounds', bounds: { ...bounds } }]
      : []
    if (settings.advance === 1) {
      const { width, descender, ascender } = document.metrics
      overlays.push({
        kind: 'line',
        from: [width, descender],
        to: [width, ascender],
      })
    }
    return overlays
  },
}

/**
 * Metrics draws the layer's baseline and zone lines across the visible region.
 *
 * It needs both context objects: the line heights come from the document's
 * resolved layer metrics, and their extent comes from the current view, so the
 * lines span exactly what the user can see at any zoom.
 */
export const metricsReporter: ReporterPlugin = {
  manifest: {
    id: 'fontcarver.metrics',
    version: '1.0.0',
    kind: 'reporter',
    nameKey: 'plugins.metricsName',
    descriptionKey: 'plugins.metricsDescription',
  },
  report: ({ document, view, settings }: ReporterInput) => {
    const { visible } = view
    if (!(visible.x1 > visible.x0)) return []
    const { xHeight, capHeight } = document.metrics
    const heights = [0, ...(settings.zones === 1 ? [xHeight, capHeight] : [])]
    return heights
      .filter(
        (height) =>
          Number.isFinite(height) &&
          height >= visible.y0 &&
          height <= visible.y1,
      )
      .map((height) => ({
        kind: 'line' as const,
        from: [visible.x0, height] as [number, number],
        to: [visible.x1, height] as [number, number],
      }))
  },
}
