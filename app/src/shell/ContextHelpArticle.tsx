import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Dialog } from '../components/Dialog'

import {
  HELP_TOPICS,
  helpHint,
  type HelpContext,
  type HelpTopic,
  useContextHelpStore,
} from './contextHelp'
import styles from './ContextHelpDialog.module.css'

/** Local help article shown above the editing workflow. */
export function ContextHelpArticle({ context }: { context: HelpContext }) {
  const { t } = useTranslation()
  const close = useContextHelpStore((state) => state.close)
  const [topic, setTopic] = useState<HelpTopic>(context.topic)

  return (
    <div className={styles.layer}>
      <Dialog
        title={t('contextHelp.title')}
        size="lg"
        scrollBody
        onCancel={close}
        footer={
          <button type="button" className={styles.close} onClick={close}>
            {t('contextHelp.close')}
          </button>
        }
      >
        {context.surfaceTitle ? (
          <div className={styles.context}>
            <span>{t('contextHelp.currentDialog')}</span>
            <p>{context.surfaceTitle}</p>
          </div>
        ) : null}
        {context.commandTitle ? (
          <aside
            className={styles.control}
            aria-label={t('contextHelp.activeCommand')}
          >
            <strong>{context.commandTitle}</strong>
            {context.commandSummary ? <p>{context.commandSummary}</p> : null}
            {context.commandBinding ? (
              <p className={styles.binding}>
                {t('contextHelp.binding', { shortcut: context.commandBinding })}
              </p>
            ) : (
              <p className={styles.binding}>{t('contextHelp.unbound')}</p>
            )}
            {context.commandReason ? (
              <p className={styles.reason}>
                {t('contextHelp.unavailableHere', {
                  reason: context.commandReason,
                })}
              </p>
            ) : null}
          </aside>
        ) : null}
        {context.toolTitle ? (
          <aside
            className={styles.control}
            aria-label={t('contextHelp.activeTool')}
          >
            <strong>{context.toolTitle}</strong>
            {context.toolHint ? <p>{context.toolHint}</p> : null}
          </aside>
        ) : null}
        {context.controlHint ? (
          <aside
            className={styles.control}
            aria-label={t('contextHelp.focusedControl')}
          >
            <strong>
              {context.controlTitle ?? t('contextHelp.focusedControl')}
            </strong>
            <p>{context.controlHint}</p>
          </aside>
        ) : null}
        <label className={styles.topicPicker}>
          <span>{t('contextHelp.topic')}</span>
          <select
            value={topic}
            {...helpHint(topic)}
            onChange={(event) => setTopic(event.target.value as HelpTopic)}
          >
            {HELP_TOPICS.map((option) => (
              <option key={option} value={option}>
                {t(`contextHelp.topics.${option}.title`)}
              </option>
            ))}
          </select>
        </label>
        <article className={styles.article} aria-live="polite">
          <h3>{t(`contextHelp.topics.${topic}.title`)}</h3>
          <p>{t(`contextHelp.topics.${topic}.summary`)}</p>
          <ul>
            <li>{t(`contextHelp.topics.${topic}.first`)}</li>
            <li>{t(`contextHelp.topics.${topic}.second`)}</li>
            <li>{t(`contextHelp.topics.${topic}.third`)}</li>
          </ul>
        </article>
        <p className={styles.hint}>{t('contextHelp.navigationHint')}</p>
      </Dialog>
    </div>
  )
}
