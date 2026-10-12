import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '../../components/Button'
import { PanelSection } from '../glyph/panels/PanelSection'
import type { PanelContext } from '../glyph/panels/panelContext'
import { PsHintEditor } from './PsHintEditor'
import { TtHintEditor } from './TtHintEditor'

export function HintPanel({
  target,
  master,
  reload,
}: Pick<PanelContext, 'target' | 'master' | 'reload'>) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [ttOpen, setTtOpen] = useState(false)
  if (!master) return null
  return (
    <PanelSection id="hints" title={t('hints.title')} defaultOpen={false}>
      <Button onClick={() => setOpen(true)}>{t('hints.psTitle')}</Button>
      <Button onClick={() => setTtOpen(true)}>{t('tt.open')}</Button>
      {ttOpen && (
        <TtHintEditor
          key={`${target.doc}:${master}:${target.glyph}:${target.layer}`}
          doc={target.doc}
          master={master}
          glyph={target.glyph}
          layer={target.layer}
          onClose={() => setTtOpen(false)}
          onApplied={reload}
        />
      )}
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
