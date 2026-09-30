import { useTranslation } from 'react-i18next'

import type { Guide } from '../../api'
import type { LayerTarget } from '../../editor/pointCommands'
import { selectionCounts, type Selection } from '../../editor/selection'
import { useEditorStore } from '../../stores/editorStore'

import { InfoGlyphRows } from './InfoGlyphRows'
import { InfoMultiRows } from './InfoMultiRows'
import { InfoObjectRows } from './InfoObjectRows'
import { InfoPointRows } from './InfoPointRows'
import type { InkEdges } from './guideAnchor'
import type { ReadableOutline } from './infoBoxProps'
import styles from './infoBox.module.css'

export interface InfoBoxProps {
  target: LayerTarget
  outline: ReadableOutline
  selection: Selection
  master: string | null
  masterGuides: readonly Guide[]
  /** Ink edges an anchored guide measures from. */
  inkEdges: InkEdges
  name: string
  unicodes: readonly number[]
  onCommitted: () => void
}

/**
 * The Info box (design spec §6.1): a docked, non-modal panel at the bottom
 * left of the canvas whose rows follow the selection. Every field commits one
 * command, so the numeric route to a change and the dragged route produce the
 * same undo history.
 */
export function InfoBox(props: InfoBoxProps) {
  const { t } = useTranslation()
  const open = useEditorStore((state) => state.infoBoxOpen)
  const toggle = useEditorStore((state) => state.toggleInfoBox)
  const counts = selectionCounts(props.selection)

  const title = (): string => {
    if (counts.total === 0) return t('infoBox.title.glyph')
    if (counts.total > 1) return t('infoBox.title.multi')
    if (counts.nodes === 1) return t('infoBox.title.node')
    if (counts.handles === 1) return t('infoBox.title.handle')
    if (counts.components === 1) return t('infoBox.title.component')
    if (counts.anchors === 1) return t('infoBox.title.anchor')
    if (counts.guides === 1) return t('infoBox.title.guide')
    return t('infoBox.title.segment')
  }

  const rows = () => {
    if (counts.total === 0) return <InfoGlyphRows {...props} />
    if (counts.total > 1) return <InfoMultiRows {...props} />
    if (counts.nodes === 1 || counts.handles === 1)
      return <InfoPointRows {...props} />
    return <InfoObjectRows {...props} />
  }

  return (
    <section
      className={styles.box}
      aria-label={t('infoBox.label')}
      data-open={open}
    >
      <header className={styles.header}>
        <h2 className={styles.title}>{title()}</h2>
        <button
          type="button"
          className={styles.collapse}
          aria-expanded={open}
          aria-label={t('infoBox.toggle')}
          title={t('infoBox.toggle')}
          onClick={toggle}
        >
          {open ? '▾' : '▸'}
        </button>
      </header>
      {open ? <div className={styles.rows}>{rows()}</div> : null}
    </section>
  )
}
