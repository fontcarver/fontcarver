import { useTranslation } from 'react-i18next'
import type { DocId, DocSummary } from '../../api'
import { documentLabel } from '../../stores/sessionStore'
import styles from './crossDocument.module.css'

/** Source document and explicit master choices shared by transfer and diff. */
export function DocumentMasterFields({
  docs,
  sourceDoc,
  sourceMaster,
  target,
  targetMaster,
  onSource,
  onSourceMaster,
  onTargetMaster,
  disabled = false,
}: {
  docs: DocSummary[]
  sourceDoc: DocId | null
  sourceMaster: string
  target: DocSummary
  targetMaster: string
  onSource: (doc: DocId) => void
  onSourceMaster: (master: string) => void
  onTargetMaster: (master: string) => void
  disabled?: boolean
}) {
  const { t } = useTranslation()
  const source = docs.find((doc) => doc.doc === sourceDoc)
  return (
    <fieldset className={styles.form} disabled={disabled}>
      <label className={styles.field}>
        {t('crossDocument.source')}
        <select
          value={sourceDoc ?? ''}
          onChange={(event) => onSource(Number(event.target.value))}
        >
          <option value="" disabled>
            {t('crossDocument.chooseSource')}
          </option>
          {docs
            .filter((doc) => doc.doc !== target.doc)
            .map((doc) => (
              <option key={doc.doc} value={doc.doc}>
                {documentLabel(doc)} · {doc.style_name}
              </option>
            ))}
        </select>
      </label>
      <p className={styles.help}>
        {t('crossDocument.target', { name: documentLabel(target) })}
      </p>
      <div className={styles.row}>
        <label className={styles.field}>
          {t('crossDocument.sourceMaster')}
          <select
            value={sourceMaster}
            onChange={(event) => onSourceMaster(event.target.value)}
          >
            {source?.masters.map((master) => (
              <option key={master.id} value={master.id}>
                {master.name}
              </option>
            ))}
          </select>
        </label>
        <label className={styles.field}>
          {t('crossDocument.targetMaster')}
          <select
            value={targetMaster}
            onChange={(event) => onTargetMaster(event.target.value)}
          >
            {target.masters.map((master) => (
              <option key={master.id} value={master.id}>
                {master.name}
              </option>
            ))}
          </select>
        </label>
      </div>
    </fieldset>
  )
}
