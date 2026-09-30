import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../../../components/Button'
import { NumberField } from '../../../components/NumberField'
import {
  spacingLockedForDocument,
  useSpacingLocked,
} from '../../text/kerningModes'
import { commitField } from '../commitField'

import type { PanelContext } from './panelContext'
import styles from './panels.module.css'

type VerticalMetricsPanelProps = Pick<
  PanelContext,
  'target' | 'glyph' | 'scene' | 'outline' | 'reload'
>

/** Vertical spacing measured from the resolved ink bounds in font coordinates. */
export function VerticalMetricsPanel({
  target,
  glyph,
  scene,
  outline,
  reload,
}: VerticalMetricsPanelProps) {
  const { t } = useTranslation()
  const [revision, setRevision] = useState(0)
  const locked = useSpacingLocked(target.doc)
  const layer = glyph.layers.find((item) => item.id === target.layer)
  const advance = scene.metrics.verticalWidth ?? scene.metrics.upem
  const origin = scene.metrics.verticalOrigin ?? scene.metrics.ascender
  const bounds = outline.bbox
  const top = bounds ? origin - bounds[3] : 0
  const bottom = bounds ? advance - origin + bounds[1] : 0

  const write = (nextAdvance: number | null, nextOrigin: number | null) => {
    if (spacingLockedForDocument(target.doc)) return
    commitField(
      target,
      {
        SetVerticalMetrics: {
          glyph: glyph.name,
          layer: target.layer,
          advance: nextAdvance,
          origin: nextOrigin,
        },
      },
      t('variation.verticalMetrics'),
      () => {
        // A rejected edit leaves the metric unchanged. Still reconcile the
        // numeric draft, which otherwise only follows a changed model value.
        setRevision((value) => value + 1)
        reload()
      },
    )
  }

  return (
    <>
      <NumberField
        label={t('glyphView.metric.verticalAdvance')}
        value={advance}
        disabled={locked || !layer}
        invalidHint={t('infoBox.invalid')}
        revision={revision}
        onCommit={(value) => write(value, layer?.vertical_origin ?? null)}
      />
      <NumberField
        label={t('glyphView.metric.verticalOrigin')}
        value={origin}
        disabled={locked || !layer}
        invalidHint={t('infoBox.invalid')}
        revision={revision}
        onCommit={(value) => write(layer?.vertical_width ?? null, value)}
      />
      <NumberField
        label={t('glyphView.metric.tsb')}
        value={top}
        disabled={locked || !layer || bounds === null}
        invalidHint={t('infoBox.invalid')}
        revision={revision}
        onCommit={(value) => {
          if (bounds) write(layer?.vertical_width ?? null, bounds[3] + value)
        }}
      />
      <NumberField
        label={t('glyphView.metric.bsb')}
        value={bottom}
        disabled={locked || !layer || bounds === null}
        invalidHint={t('infoBox.invalid')}
        revision={revision}
        onCommit={(value) => {
          if (bounds)
            write(origin - bounds[1] + value, layer?.vertical_origin ?? null)
        }}
      />
      <p className={styles.hint}>
        {t(
          bounds
            ? 'glyphView.metric.verticalBearingsHint'
            : 'glyphView.metric.emptyVerticalBearings',
        )}
      </p>
      <Button
        disabled={
          locked ||
          !layer ||
          (layer.vertical_width == null && layer.vertical_origin == null)
        }
        onClick={() => write(null, null)}
      >
        {t('variation.useMasterMetrics')}
      </Button>
    </>
  )
}
