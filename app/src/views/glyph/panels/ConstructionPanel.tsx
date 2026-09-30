import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../../../components/Button'
import { selectedNodeRefs } from '../../../editor/selection'
import { useEditorStore } from '../../../stores/editorStore'
import { ReusableConstructionEditor } from './ReusableConstructionEditor'
import { NodeLinksPanel } from './NodeLinksPanel'
import type { PanelContext } from './panelContext'
import styles from './panels.module.css'

/** Live glyph attachments, guide constraints and interpolated nodes. */
export function ConstructionPanel(context: PanelContext) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const selection = useEditorStore((state) => state.selection)
  const layer = context.glyph.layers.find(
    (entry) => entry.id === context.target.layer,
  )
  if (!layer) return null
  return (
    <>
      <p className={styles.hint}>{t('construction.description')}</p>
      <Button onClick={() => setOpen(true)}>
        {t('construction.attachments', {
          count: layer.constructions?.length ?? 0,
        })}
      </Button>
      <NodeLinksPanel {...context} />
      {open && (
        <ReusableConstructionEditor
          key={`${context.target.doc}:${layer.id}`}
          target={context.target}
          layer={layer}
          points={selectedNodeRefs(selection)}
          onClose={() => setOpen(false)}
          onApplied={context.reload}
        />
      )}
    </>
  )
}
