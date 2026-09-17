import { useTranslation } from 'react-i18next'
import styles from './kerningGroups.module.css'
import shared from './panels.module.css'

/** A bounded replacement-page control shared by group and membership queries. */
export function KerningGroupPager({
  offset,
  total,
  onPage,
}: {
  offset: number
  total: number
  onPage: (offset: number) => void
}) {
  const { t } = useTranslation()
  if (total <= 25) return null
  return (
    <div className={styles.pager}>
      <button
        type="button"
        className={shared.actionButton}
        disabled={offset === 0}
        onClick={() => onPage(Math.max(0, offset - 25))}
      >
        {t('kerningGroups.previous')}
      </button>
      <span className={shared.hint}>
        {t('kerningGroups.page', {
          first: offset + 1,
          last: Math.min(offset + 25, total),
          total,
        })}
      </span>
      <button
        type="button"
        className={shared.actionButton}
        disabled={offset + 25 >= total}
        onClick={() => onPage(offset + 25)}
      >
        {t('kerningGroups.next')}
      </button>
    </div>
  )
}
