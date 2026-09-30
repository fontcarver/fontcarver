import type { MasterMetrics } from '../../api'

/** Optional metric fields use compiler defaults until the master overrides them. */
export type OptionalMasterMetric = {
  [K in keyof MasterMetrics]: null extends MasterMetrics[K] ? K : never
}[keyof MasterMetrics]

/** Effective value and its source, matching fc-compile's instance metric policy. */
export function resolveMasterMetric(
  metrics: MasterMetrics,
  field: OptionalMasterMetric,
  unitsPerEm: number,
): {
  value: number
  source: 'master' | 'font' | 'automatic'
  overridden: boolean
} {
  const gap =
    metrics.typo_line_gap ??
    Math.max(0, unitsPerEm * 1.2 + metrics.descender - metrics.ascender)
  const defaults: Record<OptionalMasterMetric, number> = {
    hhea_ascender: metrics.ascender + gap,
    hhea_descender: metrics.descender,
    hhea_line_gap: 0,
    typo_ascender: metrics.ascender,
    typo_descender: metrics.descender,
    typo_line_gap: gap,
    win_ascent: metrics.ascender + gap,
    win_descent: Math.abs(metrics.descender),
    vhea_ascender: 0,
    vhea_descender: 0,
    vhea_line_gap: 0,
    default_vertical_origin: metrics.ascender,
    default_vertical_advance: unitsPerEm,
  }
  const source =
    field === 'default_vertical_advance'
      ? 'font'
      : [
            'hhea_line_gap',
            'vhea_ascender',
            'vhea_descender',
            'vhea_line_gap',
          ].includes(field)
        ? 'automatic'
        : 'master'
  return {
    value: metrics[field] ?? defaults[field],
    source,
    overridden: metrics[field] !== null,
  }
}
