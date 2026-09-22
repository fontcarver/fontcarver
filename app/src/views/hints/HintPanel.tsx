import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '../../components/Button'
import { PanelSection } from '../glyph/panels/PanelSection'
import type { PanelContext } from '../glyph/panels/panelContext'
import { PsHintEditor } from './PsHintEditor'

export function HintPanel({
  target,
  master,
  reload,
}: Pick<PanelContext, 'target' | 'master' | 'reload'>) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  if (!master) return null
  return (
    <PanelSection id="hints" title={t('hints.title')} defaultOpen={false}>
      <Button onClick={() => setOpen(true)}>{t('hints.psTitle')}</Button>
      {open && (
        <PsHintEditor
          doc={target.doc}
          master={master}
          glyph={target.glyph}
          layer={target.layer}
          onClose={() => setOpen(false)}
          onApplied={reload}
        />
      )}
    </PanelSection>
  )
}
