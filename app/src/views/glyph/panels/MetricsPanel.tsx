import { useTranslation } from 'react-i18next'

import type { MetricsKeys } from '../../../api'
import { NumberField } from '../../../components/NumberField'
import { setMetricsCommand } from '../../../editor/pointCommands'
import { commitField } from '../commitField'

import styles from './panels.module.css'
import type { PanelContext } from './panelContext'
import { metricsKeysWith, setMetricsKeysCommand } from './panelCommands'
import { TextRow } from './TextRow'
import { outOfSync, useResolvedMetrics } from './useResolvedMetrics'

/** The three horizontal formulas, in the order the panel shows them. */
const KEY_FIELDS: readonly {
  field: keyof MetricsKeys
  label: string
  hint: string
}[] = [
  { field: 'left', label: 'glyphView.metric.lsb', hint: '=H' },
  { field: 'right', label: 'glyphView.metric.rsb', hint: '=|n' },
  { field: 'width', label: 'glyphView.metric.advance', hint: '=n+10' },
]

function round(value: number): string {
  return String(Math.round(value * 10) / 10)
}

/**
 * The Metrics section (design spec §7.2): the layer's own width and
 * sidebearings, the metrics formulas, the value they resolve to right now and
 * an Apply that writes that value into the layer.
 */
export function MetricsPanel({
  target,
  gid,
  glyph,
  scene,
  outline,
  master,
  reload,
}: PanelContext) {
  const { t } = useTranslation()
  const keys = glyph.metrics_keys
  const stored = { lsb: outline.lsb, rsb: outline.rsb, width: outline.width }
  const resolved = useResolvedMetrics(
    target.doc,
    gid,
    master,
    keys,
    outline.resolved_metrics,
  )
  const stale = outOfSync(resolved, stored)

  const write = (values: { width?: number; lsb?: number; rsb?: number }) =>
    commitField(
      target,
      setMetricsCommand(target, values),
      t('editor.undo.setMetrics'),
      reload,
    )

  const writeKeys = (next: MetricsKeys) =>
    commitField(
      target,
      setMetricsKeysCommand(glyph.name, next),
      t('editor.undo.setMetricsKeys'),
      reload,
    )

  return (
    <>
      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('glyphInspector.layer')}</span>
        <span className={styles.rowValue}>{scene.layerName || '—'}</span>
      </div>
      <NumberField
        label={t('glyphView.metric.advance')}
        value={outline.width}
        invalidHint={t('infoBox.invalid')}
        onCommit={(width) => write({ width })}
      />
      <NumberField
        label={t('glyphView.metric.lsb')}
        value={outline.lsb}
        invalidHint={t('infoBox.invalid')}
        onCommit={(lsb) => write({ lsb })}
      />
      <NumberField
        label={t('glyphView.metric.rsb')}
        value={outline.rsb}
        invalidHint={t('infoBox.invalid')}
        onCommit={(rsb) => write({ rsb })}
      />
      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('glyphInspector.bbox')}</span>
        <span className={styles.rowValue}>
          {outline.bbox
            ? `${round(outline.bbox[0])} ${round(outline.bbox[1])} → ${round(
                outline.bbox[2],
              )} ${round(outline.bbox[3])}`
            : '—'}
        </span>
      </div>

      {KEY_FIELDS.map((entry) => (
        <TextRow
          key={entry.field}
          label={t('panel.metrics.key', { metric: t(entry.label) })}
          value={keys[entry.field] ?? ''}
          placeholder={entry.hint}
          onCommit={(value) =>
            writeKeys(metricsKeysWith(keys, entry.field, value))
          }
        />
      ))}

      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('panel.metrics.resolved')}</span>
        <span className={styles.rowValue}>
          {`${round(resolved.lsb)} / ${round(resolved.width)} / ${round(
            resolved.rsb,
          )}`}
          {stale ? (
            <>
              {' '}
              <span className={styles.warning}>
                {t('panel.metrics.outOfSync')}
              </span>
            </>
          ) : null}
        </span>
      </div>
      <div className={styles.actions}>
        <button
          type="button"
          className={styles.actionButton}
          disabled={!stale}
          onClick={() =>
            write({
              width: resolved.width,
              lsb: resolved.lsb,
              rsb: resolved.rsb,
            })
          }
        >
          {t('panel.metrics.apply')}
        </button>
      </div>

      {scene.metrics.verticalOrigin === null ? null : (
        <div className={styles.row}>
          <span className={styles.rowLabel}>
            {t('glyphView.metric.verticalOrigin')}
          </span>
          <span className={styles.rowValue}>
            {round(scene.metrics.verticalOrigin)}
          </span>
        </div>
      )}
      {scene.metrics.verticalWidth === null ? null : (
        <div className={styles.row}>
          <span className={styles.rowLabel}>
            {t('glyphView.metric.verticalAdvance')}
          </span>
          <span className={styles.rowValue}>
            {round(scene.metrics.verticalWidth)}
          </span>
        </div>
      )}
    </>
  )
}
