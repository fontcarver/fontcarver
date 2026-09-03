import { useTranslation } from 'react-i18next'

import type { DocSummary, GlyphId } from '../../api'
import { Field } from '../../components/Field'
import styles from '../placeholder.module.css'

export interface GlyphViewPlaceholderProps {
  summary: DocSummary
  gid: GlyphId | undefined
}

/** Stand-in for the Glyph view until the outline canvas lands. */
export function GlyphViewPlaceholder({
  summary,
  gid,
}: GlyphViewPlaceholderProps) {
  const { t } = useTranslation()
  return (
    <div className={styles.placeholder}>
      <h2 className={styles.placeholderTitle}>
        {t('view.glyphLabel', { gid: gid ?? '—' })}
      </h2>
      <p className={styles.placeholderNote}>{t('view.glyph.note')}</p>
      <div className={styles.summaryGrid}>
        <Field label={t('inspector.family')} readOnly>
          {summary.family_name}
        </Field>
      </div>
    </div>
  )
}
