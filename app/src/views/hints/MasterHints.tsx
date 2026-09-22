import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { DocId, Master } from '../../api'
import { Button } from '../../components/Button'
import { PsHintEditor } from './PsHintEditor'
import styles from './hints.module.css'

export function MasterHints({
  doc,
  masters,
}: {
  doc: DocId
  masters: readonly Master[]
}) {
  const { t } = useTranslation()
  const [selected, setSelected] = useState<string>()
  return (
    <div className={styles.row}>
      {masters.map((master) => (
        <Button key={master.id} onClick={() => setSelected(master.id)}>
          {t('hints.masterParameters', { master: master.name })}
        </Button>
      ))}
      {selected && (
        <PsHintEditor
          key={selected}
          doc={doc}
          master={selected}
          onClose={() => setSelected(undefined)}
          onApplied={() => setSelected(undefined)}
        />
      )}
    </div>
  )
}
