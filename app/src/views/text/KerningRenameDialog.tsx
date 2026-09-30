import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { KerningGroupSide } from '../../api'
import { Dialog } from '../../components/Dialog'
import { canonicalGroupName } from '../glyph/panels/kerningGroupEdits'
import styles from './KerningTable.module.css'

/** Confirms the document-wide scope of a canonical kerning group rename. */
export function KerningRenameDialog({
  name,
  side,
  busy,
  onCancel,
  onRename,
}: {
  name: string
  side: KerningGroupSide
  busy: boolean
  onCancel: () => void
  onRename: (name: string) => void
}) {
  const { t } = useTranslation()
  const [draft, setDraft] = useState(name)
  const target = canonicalGroupName(side, draft)
  const valid = draft.trim().length > 0 && target !== name
  const confirm = () => {
    if (valid && !busy) onRename(target)
  }
  return (
    <Dialog
      title={t('kerningTable.renameTitle')}
      onCancel={onCancel}
      onConfirm={confirm}
      footer={
        <>
          <button type="button" disabled={busy} onClick={onCancel}>
            {t('common.cancel')}
          </button>
          <button type="button" disabled={busy || !valid} onClick={confirm}>
            {t('kerningTable.renameConfirm')}
          </button>
        </>
      }
    >
      <p>{t('kerningTable.renameScope', { name })}</p>
      <label className={styles.renameName}>
        {t('kerningTable.renameName')}
        <input
          value={draft}
          onChange={(event) => setDraft(event.currentTarget.value)}
          disabled={busy}
        />
      </label>
    </Dialog>
  )
}
