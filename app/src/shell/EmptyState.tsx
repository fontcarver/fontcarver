import { useId } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../components/Button'
import { useSessionStore } from '../stores/sessionStore'
import { useKeymapStore } from '../stores/keymapStore'

import { runCommand } from './commands'
import { openHelpArticle } from './contextHelp'
import { shortcutFor } from './keymap'
import { formatShortcut } from './shortcuts'
import { useCommandLookup } from './useCommandRegistry'
import styles from './EmptyState.module.css'

/** Shown in the content area while no document is open. */
export function EmptyState() {
  const { t } = useTranslation()
  const recent = useSessionStore((state) => state.recent)
  const openDialog = useSessionStore((state) => state.openDialog)
  const open = useSessionStore((state) => state.open)
  const overrides = useKeymapStore((state) => state.overrides)
  const discoveryId = useId()
  // Read through the lookup, not the registry itself: a late registration
  // must reach the hints below.
  const commandById = useCommandLookup()

  const discoveries = [
    {
      label: 'contextHelp.topics.welcome.title',
      commands: [],
      run: () => openHelpArticle('welcome'),
    },
    {
      label: 'cmd.palette.open',
      commands: ['palette.open', 'palette.quickOpen'],
      run: () => runCommand('palette.open'),
    },
    {
      label: 'cmd.help.shortcuts',
      commands: ['help.shortcuts'],
      run: () => runCommand('help.shortcuts'),
    },
  ]

  return (
    <div className={styles.empty}>
      <div className={styles.emptyCard}>
        <h1 className={styles.emptyTitle}>{t('app.name')}</h1>
        <p className={styles.emptyTagline}>{t('empty.tagline')}</p>
        <div className={styles.emptyActions}>
          <Button variant="primary" large onClick={() => void openDialog()}>
            {t('empty.open')}
          </Button>
          <Button large onClick={() => runCommand('file.new')}>
            {t('empty.new')}
          </Button>
        </div>
        <div className={styles.emptyRecent}>
          <h2 className={styles.emptyRecentTitle}>{t('empty.recent')}</h2>
          {recent.length === 0 ? (
            <p className={styles.noRecent}>{t('empty.noRecent')}</p>
          ) : (
            <ul className={styles.recentList}>
              {recent.map((path) => (
                <li key={path}>
                  <button
                    type="button"
                    title={path}
                    className={styles.recentItem}
                    onClick={() => void open(path)}
                  >
                    {path}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
        <section className={styles.discovery} aria-labelledby={discoveryId}>
          <h2 id={discoveryId} className={styles.emptyRecentTitle}>
            {t('empty.explore')}
          </h2>
          {discoveries.map(({ label, commands, run }) => {
            const bindings = [
              ...new Set(
                commands.flatMap((id) => {
                  const binding = shortcutFor(
                    id,
                    commandById(id)?.shortcut,
                    overrides,
                  )
                  const hint = binding ? formatShortcut(binding) : ''
                  return hint ? [hint] : []
                }),
              ),
            ]
            return (
              <button
                key={label}
                type="button"
                className={styles.discoveryItem}
                onClick={run}
              >
                <span>{t(label)}</span>
                {bindings.length > 0 ? (
                  <span className={styles.bindings}>
                    {bindings.map((binding) => (
                      <kbd key={binding}>{binding}</kbd>
                    ))}
                  </span>
                ) : null}
              </button>
            )
          })}
        </section>
      </div>
    </div>
  )
}
