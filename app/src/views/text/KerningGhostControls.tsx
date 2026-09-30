import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import type { KerningGhostModel } from './useKerningGhosts'
import styles from './KerningGhostControls.module.css'

/** Toggle and signed feedback supplement the canvas's visual annotations. */
export function KerningGhostControls({ model }: { model: KerningGhostModel }) {
  const { t } = useTranslation()
  const hint = useId()
  const sign =
    model.value === null
      ? null
      : model.value < 0
        ? 'negative'
        : model.value > 0
          ? 'positive'
          : 'zero'
  return (
    <div className={styles.row}>
      <label className={styles.toggle}>
        <input
          type="checkbox"
          checked={model.enabled}
          disabled={!model.available && !model.enabled}
          aria-describedby={hint}
          onChange={(event) => model.setEnabled(event.currentTarget.checked)}
        />
        {t('kerningGhosts.toggle')}
      </label>
      <span id={hint} className={styles.hint}>
        {t(
          model.available ? 'kerningGhosts.help' : 'kerningGhosts.unavailable',
        )}
      </span>
      <span role="status" className={styles.hint}>
        {model.enabled &&
          (model.loading
            ? t('kerningGhosts.loading')
            : model.failed
              ? t('kerningGhosts.failed')
              : model.available
                ? t('kerningGhosts.count', { count: model.outlines.length })
                : '')}
      </span>
      {sign && (
        <span data-sign={sign} className={styles.value}>
          {t(`kerningGhosts.${sign}`, {
            value: model.value! > 0 ? `+${model.value}` : model.value,
          })}
        </span>
      )}
    </div>
  )
}
