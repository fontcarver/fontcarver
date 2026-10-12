import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../components/Button'
import { Dialog } from '../components/Dialog'
import { useUiStore } from '../stores/uiStore'
import { Inspector } from './Inspector'
import { Sidebar } from './Sidebar'
import styles from './panels.module.css'

/** Keep the editor usable when zoom leaves too little room for three columns. */
export function useCompactPanels() {
  useEffect(() => {
    const query = globalThis.matchMedia?.('(max-width: 900px)')
    if (!query) return
    const update = () => useUiStore.getState().setCompactPanels(query.matches)
    update()
    query.addEventListener('change', update)
    return () => query.removeEventListener('change', update)
  }, [])
}

/** A temporary sidebar/inspector with the shared focus trap and Escape behavior. */
export function CompactPanel() {
  const { t } = useTranslation()
  const panel = useUiStore((state) => state.compactPanel)
  if (!panel) return null
  const close = () => {
    const state = useUiStore.getState()
    if (state.compactPanel === panel) state.togglePanel(panel)
  }
  return (
    <Dialog
      title={t(panel === 'sidebar' ? 'a11y.sidebar' : 'a11y.inspector')}
      onCancel={close}
      footer={<Button onClick={close}>{t('common.close')}</Button>}
    >
      <div className={styles.compactPanel}>
        {panel === 'sidebar' ? <Sidebar /> : <Inspector />}
      </div>
    </Dialog>
  )
}
