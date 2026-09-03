import { useTranslation } from 'react-i18next'

import type { DocSummary } from '../../api'
import { Field } from '../../components/Field'
import styles from '../../shell/shell.module.css'

export interface FontInfoPlaceholderProps {
  summary: DocSummary
}

/** Stand-in for the Font Info tab until the real panel lands. */
export function FontInfoPlaceholder({ summary }: FontInfoPlaceholderProps) {
  const { t } = useTranslation()
  return (
    <div className={styles.placeholder}>
      <h2 className={styles.placeholderTitle}>{t('view.fontinfo.title')}</h2>
      <p className={styles.placeholderNote}>{t('view.fontinfo.note')}</p>
      <div className={styles.summaryGrid}>
        <Field label={t('inspector.family')} readOnly>
          {summary.family_name}
        </Field>
        <Field label={t('inspector.style')} readOnly>
          {summary.style_name}
        </Field>
        <Field label={t('inspector.format')} readOnly>
          {summary.format}
        </Field>
        <Field label={t('inspector.masters')} readOnly>
          {summary.masters.map((master) => master.name).join(', ') || '—'}
        </Field>
        <Field label={t('inspector.axes')} readOnly>
          {summary.axes.map((axis) => axis.tag).join(', ') || '—'}
        </Field>
      </div>
    </div>
  )
}
