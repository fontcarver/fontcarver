import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { MetricsKeys } from '../../../api'
import { NumberField } from '../../../components/NumberField'
import { setMetricsCommand } from '../../../editor/pointCommands'
import { commitField } from '../commitField'
import {
  spacingLockedForDocument,
  useSpacingLocked,
} from '../../text/kerningModes'

import styles from './panels.module.css'
import type { PanelContext } from './panelContext'
import {
  displayedMetricsKey,
  effectiveMetricsKeys,
  metricsKeyCommand,
} from './metricsKeyCommands'
import { TextRow } from './TextRow'
import { VerticalMetricsPanel } from './VerticalMetricsPanel'
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
 * commands that resolve and update the selected master or all masters.
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
  const [revision, setRevision] = useState(0)
  const locked = useSpacingLocked(target.doc)
  const layer = glyph.layers.find((layer) => layer.id === target.layer)
  const currentMaster =
    layer &&
    typeof layer.kind === 'object' &&
    'Master' in layer.kind &&
    layer.kind.Master === master
      ? master
      : null
  const keys = effectiveMetricsKeys(glyph.metrics_keys, layer?.metrics_keys)
  const stored = { lsb: outline.lsb, rsb: outline.rsb, width: outline.width }
  const resolution = useResolvedMetrics(
    target.doc,
    gid,
    master,
    keys,
    outline.resolved_metrics,
    target.layer,
  )
  const resolved = resolution.value
  const stale = resolution.status === 'ready' && outOfSync(resolved, stored)
  const reconcile = () => {
    setRevision((value) => value + 1)
    reload()
  }

  const write = (values: { width?: number; lsb?: number; rsb?: number }) => {
    if (spacingLockedForDocument(target.doc)) return
    commitField(
      target,
      setMetricsCommand(target, values),
      t('editor.undo.setMetrics'),
      reconcile,
    )
  }

  const writeKeys = (field: keyof MetricsKeys, value: string) => {
    if (spacingLockedForDocument(target.doc)) return
    commitField(
      target,
      metricsKeyCommand(
        glyph.name,
        target.layer,
        glyph.metrics_keys,
        layer?.metrics_keys,
        field,
        value,
      ),
      t('editor.undo.setMetricsKeys'),
      reconcile,
    )
  }

  const update = (master: string | null) => {
    if (spacingLockedForDocument(target.doc)) return
    commitField(
      target,
      { UpdateMetrics: { glyph: glyph.name, master } },
      t('panel.metrics.update'),
      reconcile,
    )
  }

  return (
    <>
      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('glyphInspector.layer')}</span>
        <span className={styles.rowValue}>{scene.layerName || '—'}</span>
      </div>
      <NumberField
        revision={revision}
        disabled={locked}
        label={t('glyphView.metric.advance')}
        value={outline.width}
        invalidHint={t('infoBox.invalid')}
        onCommit={(width) => write({ width })}
      />
      <NumberField
        revision={revision}
        disabled={locked}
        label={t('glyphView.metric.lsb')}
        value={outline.lsb}
        invalidHint={t('infoBox.invalid')}
        onCommit={(lsb) => write({ lsb })}
      />
      <NumberField
        revision={revision}
        disabled={locked}
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
          disabled={locked}
          revision={revision}
          label={t('panel.metrics.key', { metric: t(entry.label) })}
          value={displayedMetricsKey(
            glyph.metrics_keys,
            layer?.metrics_keys,
            entry.field,
          )}
          placeholder={entry.hint}
          onCommit={(value) => writeKeys(entry.field, value)}
        />
      ))}
      <p className={styles.hint}>{t('panel.metrics.scopeHint')}</p>
      {locked && (
        <p className={styles.hint}>{t('kerningModes.spacingLocked')}</p>
      )}

      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('panel.metrics.resolved')}</span>
        <span className={styles.rowValue}>
          {`${round(resolved.lsb)} / ${round(resolved.width)} / ${round(
            resolved.rsb,
          )}`}
        </span>
      </div>
      {resolution.status === 'error' ? (
        <p className={styles.hint} role="alert">
          {t('panel.metrics.resolveError', { message: resolution.error })}
        </p>
      ) : (
        <p className={stale ? styles.warning : styles.hint} role="status">
          {t(
            resolution.status === 'pending'
              ? 'panel.metrics.resolving'
              : stale
                ? 'panel.metrics.outOfSync'
                : 'panel.metrics.inSync',
          )}
        </p>
      )}
      <div className={styles.actions}>
        <button
          type="button"
          className={styles.actionButton}
          disabled={locked || !stale || currentMaster == null}
          onClick={() => {
            if (currentMaster != null) update(currentMaster)
          }}
        >
          {t('panel.metrics.updateCurrent')}
        </button>
        <button
          type="button"
          className={styles.actionButton}
          disabled={
            locked ||
            !glyph.layers.some(
              (entry) =>
                typeof entry.kind === 'object' && 'Master' in entry.kind,
            )
          }
          onClick={() => update(null)}
        >
          {t('panel.metrics.updateAll')}
        </button>
      </div>

      <VerticalMetricsPanel
        target={target}
        glyph={glyph}
        scene={scene}
        outline={outline}
        reload={reload}
      />
    </>
  )
}
