/**
 * Translated labels for the metric lines drawn on the canvas.
 *
 * The drawing modules take plain strings, so the translations are resolved
 * once per language change rather than inside the render loop.
 */

import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import type { MetricLabels } from './drawMetrics'

/** The metric line labels in the current language. */
export function useMetricLabels(): MetricLabels {
  const { t } = useTranslation()
  return useMemo(
    () => ({
      baseline: t('glyphView.metric.baseline'),
      xHeight: t('glyphView.metric.xHeight'),
      capHeight: t('glyphView.metric.capHeight'),
      ascender: t('glyphView.metric.ascender'),
      descender: t('glyphView.metric.descender'),
      origin: t('glyphView.metric.origin'),
      advance: t('glyphView.metric.advance'),
      lsb: t('glyphView.metric.lsb'),
      rsb: t('glyphView.metric.rsb'),
      verticalOrigin: t('glyphView.metric.verticalOrigin'),
      verticalAdvance: t('glyphView.metric.verticalAdvance'),
    }),
    // `t` changes identity when the language changes, which is the trigger.
    [t],
  )
}
