import { useTranslation } from 'react-i18next'

import type { Command } from './commands'
import { formatShortcut } from './shortcuts'
import styles from './shell.module.css'

export interface MenuItemProps {
  command: Command
  onRun: () => void
  onKeyDown: (event: React.KeyboardEvent) => void
}

/** One row of a menu popup: title, optional detail and shortcut hint. */
export function MenuItem({ command, onRun, onKeyDown }: MenuItemProps) {
  const { t } = useTranslation()
  const enabled = command.when?.() ?? true
  const detail = command.detail?.()
  const label = detail ? `${t(command.title)} ${detail}` : t(command.title)

  return (
    <button
      type="button"
      role="menuitem"
      disabled={!enabled}
      className={styles.menuItem}
      onClick={onRun}
      onKeyDown={onKeyDown}
    >
      <span className={styles.menuItemLabel}>{label}</span>
      {command.shortcut ? (
        <span className={styles.shortcut}>
          {formatShortcut(command.shortcut)}
        </span>
      ) : null}
    </button>
  )
}
