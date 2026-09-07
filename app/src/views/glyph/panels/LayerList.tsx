import { useTranslation } from 'react-i18next'

import type { DocId, Layer, MasterSummary } from '../../../api'
import { useDocumentStore } from '../../../stores/documentStore'
import { useEditorStore } from '../../../stores/editorStore'

import styles from './panels.module.css'
import { layerDetail, layerLabel, layerRows, type LayerRow } from './layerRows'

export interface LayerListProps {
  layers: readonly Layer[]
  masters: readonly MasterSummary[]
  doc: DocId
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
  doc,
  current,
  rowActions,
}: LayerListProps) {
  const { t } = useTranslation()
  const setEditedLayer = useEditorStore((state) => state.setEditedLayer)
  const setMaster = useDocumentStore((state) => state.setMaster)
  const rows = layerRows(layers)

  if (rows.length === 0)
    return <p className={styles.empty}>{t('glyphInspector.noLayers')}</p>

  return (
    <div className={styles.list} role="list">
      {rows.map((row) => {
        const active = row.layer.id === current
        const role = t(`glyphInspector.layerKind.${row.kindKey}`)
        const detail = layerDetail(row, masters, role, timeLabel)
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
              onClick={() => {
                setEditedLayer(row.layer.id)
                if (row.master !== null) setMaster(doc, row.master)
              }}
              title={`${role} · ${detail}`}
            >
              {layerLabel(row, masters, role)}
            </button>
            <span className={styles.rowMeta}>{detail}</span>
            {rowActions?.(row)}
          </div>
        )
      })}
    </div>
  )
}
