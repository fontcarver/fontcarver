import { useTranslation } from 'react-i18next'

import type { Layer, MasterSummary } from '../../../api'
import { useEditorStore } from '../../../stores/editorStore'

import styles from './panels.module.css'
import { layerLabel, layerRows, type LayerRow } from './layerRows'

export interface LayerListProps {
  layers: readonly Layer[]
  masters: readonly MasterSummary[]
  /** The layer the canvas is editing. */
  current: string
  /** Rendered at the right of each row; the panel adds its actions here. */
  rowActions?: (row: LayerRow) => React.ReactNode
}

/** Backup timestamps are seconds since the epoch; show them as a local time. */
function timeLabel(seconds: number): string {
  return new Date(seconds * 1000).toLocaleString()
}

/**
 * The list of a glyph's layers, shared by the Layers panel and the Edit tab's
 * sidebar (design spec §3 and §7.3). Clicking a row puts that layer on the
 * canvas, which is the same store field the outline hook reads.
 */
export function LayerList({
  layers,
  masters,
  current,
  rowActions,
}: LayerListProps) {
  const { t } = useTranslation()
  const setEditedLayer = useEditorStore((state) => state.setEditedLayer)
  const rows = layerRows(layers)

  if (rows.length === 0)
    return <p className={styles.empty}>{t('glyphInspector.noLayers')}</p>

  return (
    <div className={styles.list} role="list">
      {rows.map((row) => {
        const active = row.layer.id === current
        const role = t(`glyphInspector.layerKind.${row.kindKey}`)
        return (
          <div
            key={row.layer.id}
            role="listitem"
            className={`${styles.listRow} ${active ? styles.listRowActive : ''}`}
          >
            <button
              type="button"
              className={`${styles.rowName} ${
                row.group === 'master' ? styles.rowNameStrong : ''
              }`}
              aria-current={active}
              onClick={() => setEditedLayer(row.layer.id)}
              title={row.time === null ? role : timeLabel(row.time)}
            >
              {layerLabel(row, masters, role)}
            </button>
            <span className={styles.rowMeta}>
              {row.time === null ? role : timeLabel(row.time)}
            </span>
            {rowActions?.(row)}
          </div>
        )
      })}
    </div>
  )
}
