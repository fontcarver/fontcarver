import { useRef, useState } from 'react'
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

  /**
   * The override pair this panel last sent, until the model reports it back.
   *
   * `SetVerticalMetrics` writes both fields, so each field sends the stored
   * value of the other one. A second edit made before the first command's
   * reload has landed would otherwise read the *old* record and clear what the
   * first edit just wrote, which is how typing an advance and then a TSB used
   * to lose the advance.
   */
  const sent = useRef<{
    key: string
    advance: number | null
    origin: number | null
  } | null>(null)
  const key = `${target.doc}:${glyph.name}:${target.layer}`

  /**
   * The overrides the next command starts from. Called from the fields, never
   * during render: the pending pair is dropped as soon as the model reports it
   * back, or as soon as another glyph or layer is shown.
   */
  const basis = (): { advance: number | null; origin: number | null } => {
    const stored = {
      advance: layer?.vertical_width ?? null,
      origin: layer?.vertical_origin ?? null,
    }
    const pending = sent.current
    if (!pending || pending.key !== key) return stored
    if (
      pending.advance === stored.advance &&
      pending.origin === stored.origin
    ) {
      sent.current = null
      return stored
    }
    return { advance: pending.advance, origin: pending.origin }
  }

  const write = (nextAdvance: number | null, nextOrigin: number | null) => {
    if (spacingLockedForDocument(target.doc)) return
    sent.current = { key, advance: nextAdvance, origin: nextOrigin }
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
        onCommit={(value) => write(value, basis().origin)}
      />
      <NumberField
        label={t('glyphView.metric.verticalOrigin')}
        value={origin}
        disabled={locked || !layer}
        invalidHint={t('infoBox.invalid')}
        revision={revision}
        onCommit={(value) => write(basis().advance, value)}
      />
      <NumberField
        label={t('glyphView.metric.tsb')}
        value={top}
        disabled={locked || !layer || bounds === null}
        invalidHint={t('infoBox.invalid')}
        revision={revision}
        onCommit={(value) => {
          if (bounds) write(basis().advance, bounds[3] + value)
        }}
      />
      <NumberField
        label={t('glyphView.metric.bsb')}
        value={bottom}
        disabled={locked || !layer || bounds === null}
        invalidHint={t('infoBox.invalid')}
        revision={revision}
        onCommit={(value) => {
          if (bounds) write(origin - bounds[1] + value, basis().origin)
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
