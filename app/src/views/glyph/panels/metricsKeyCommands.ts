import type { Command, MetricsKeys } from '../../../api'

import { metricsKeysWith, setMetricsKeysCommand } from './panelCommands'

const EMPTY_KEYS: MetricsKeys = {
  left: null,
  right: null,
  width: null,
  vertical_origin: null,
  vertical_width: null,
}

/** Layer formulas override only the fields they explicitly supply. */
export function effectiveMetricsKeys(
  glyphKeys: MetricsKeys,
  layerKeys: MetricsKeys | null | undefined,
): MetricsKeys {
  return {
    left: layerKeys?.left ?? glyphKeys.left,
    right: layerKeys?.right ?? glyphKeys.right,
    width: layerKeys?.width ?? glyphKeys.width,
    vertical_origin: layerKeys?.vertical_origin ?? glyphKeys.vertical_origin,
    vertical_width: layerKeys?.vertical_width ?? glyphKeys.vertical_width,
  }
}

/** Show local formulas with the scope prefix, including imported formulas. */
export function displayedMetricsKey(
  glyphKeys: MetricsKeys,
  layerKeys: MetricsKeys | null | undefined,
  field: keyof MetricsKeys,
): string {
  const local = layerKeys?.[field]
  if (local == null) return glyphKeys[field] ?? ''
  if (local.startsWith('==')) return local
  return local.startsWith('=') ? `=${local}` : `==${local}`
}

/** Edit one formula's scope without changing any other formula or override. */
export function metricsKeyCommand(
  glyph: string,
  layer: string,
  glyphKeys: MetricsKeys,
  layerKeys: MetricsKeys | null | undefined,
  field: keyof MetricsKeys,
  value: string,
): Command {
  const text = value.trim()
  const local = (next: MetricsKeys): Command => ({
    SetLayerMetricsKeys: {
      glyph,
      layer,
      keys: Object.values(next).every((key) => key == null) ? null : next,
    },
  })
  if (text.startsWith('==') || (text === '' && layerKeys?.[field] != null))
    return local(metricsKeysWith(layerKeys ?? EMPTY_KEYS, field, text))

  const global = setMetricsKeysCommand(
    glyph,
    metricsKeysWith(glyphKeys, field, text),
  )
  if (layerKeys?.[field] == null) return global
  return { Batch: [global, local(metricsKeysWith(layerKeys, field, ''))] }
}
