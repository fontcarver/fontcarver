import { useTranslation } from 'react-i18next'

import { NumberField } from '../../components/NumberField'
import { setMetricsCommand } from '../../editor/pointCommands'

import { formatUnicodes } from './glyphScene'
import { commitField } from './commitField'
import styles from './infoBox.module.css'
import type { InfoRowsProps } from './infoBoxProps'

export interface InfoGlyphRowsProps extends InfoRowsProps {
  name: string
  unicodes: readonly number[]
}

/**
 * The Info box with nothing selected (design spec §6.1): the glyph's identity
 * and its horizontal metrics. Width, LSB and RSB commit through `SetMetrics`,
 * one undo step each; the name and Unicode are read-only here and are edited
 * in the Glyph panel of the inspector.
 */
export function InfoGlyphRows({
  target,
  outline,
  name,
  unicodes,
  onCommitted,
}: InfoGlyphRowsProps) {
  const { t } = useTranslation()
  const metrics = outline.resolved_metrics
  const write = (values: { width?: number; lsb?: number; rsb?: number }) =>
    commitField(
      target,
      setMetricsCommand(target, values),
      t('editor.undo.setMetrics'),
      onCommitted,
    )

  return (
    <>
      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('infoBox.name')}</span>
        <span className={styles.rowValue}>{name}</span>
      </div>
      <div className={styles.row}>
        <span className={styles.rowLabel}>{t('infoBox.unicode')}</span>
        <span className={styles.rowValue}>{formatUnicodes(unicodes)}</span>
      </div>
      <NumberField
        label={t('glyphView.metric.advance')}
        value={metrics.width}
        invalidHint={t('infoBox.invalid')}
        onCommit={(width) => write({ width })}
      />
      <NumberField
        label={t('glyphView.metric.lsb')}
        value={metrics.lsb}
        invalidHint={t('infoBox.invalid')}
        onCommit={(lsb) => write({ lsb })}
      />
      <NumberField
        label={t('glyphView.metric.rsb')}
        value={metrics.rsb}
        invalidHint={t('infoBox.invalid')}
        onCommit={(rsb) => write({ rsb })}
      />
    </>
  )
}
