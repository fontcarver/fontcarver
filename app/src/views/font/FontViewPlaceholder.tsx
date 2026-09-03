import { useTranslation } from 'react-i18next'

import type { DocSummary } from '../../api'
import { Field } from '../../components/Field'
import styles from '../placeholder.module.css'

export interface FontViewPlaceholderProps {
  summary: DocSummary
}

/** Stand-in for the Font view until the virtualised glyph grid lands. */
export function FontViewPlaceholder({ summary }: FontViewPlaceholderProps) {
  const { t } = useTranslation()
  return (
    <div className={styles.placeholder}>
      <h2 className={styles.placeholderTitle}>{t('view.font.title')}</h2>
      <p className={styles.placeholderNote}>{t('view.font.note')}</p>
      <div className={styles.summaryGrid}>
        <Field label={t('inspector.family')} readOnly>
          {summary.family_name}
        </Field>
        <Field label={t('inspector.style')} readOnly>
          {summary.style_name}
        </Field>
        <Field label={t('inspector.glyphs')} readOnly>
          {summary.glyph_count.toLocaleString()}
        </Field>
        <Field label={t('inspector.unitsPerEm')} readOnly>
          {summary.units_per_em}
        </Field>
      </div>
    </div>
  )
}
