import { useTranslation } from 'react-i18next'

import type { Command } from './commands'
import styles from './MenuBar.module.css'
import { formatShortcut } from './shortcuts'
import { shortcutFor } from './keymap'
import { useKeymapStore } from '../stores/keymapStore'
import { commandHelp } from './commandHelp'

export interface MenuItemProps {
  command: Command
  onRun: () => void
  onKeyDown: (event: React.KeyboardEvent) => void
}

/**
 * One row of a menu popup. Disabled commands stay listed and explain why they
 * are unavailable through an inline hint and their accessible description;
 * hovering a row is Quick Help's job, so there is no native tooltip.
 */
export function MenuItem({ command, onRun, onKeyDown }: MenuItemProps) {
  const { t } = useTranslation()
  const overrides = useKeymapStore((state) => state.overrides)
  const shortcut = shortcutFor(command.id, command.shortcut, overrides)
  const help = commandHelp(command.id)
  const summary = help ? t(help.summary) : undefined
  const enabled = command.when?.() ?? true
  const detail = command.detail?.()
  const label = detail ? `${t(command.title)} ${detail}` : t(command.title)
  const reason =
    !enabled && command.disabledReason ? t(command.disabledReason) : undefined

  return (
    <button
      type="button"
      role="menuitem"
      disabled={!enabled}
      data-help-command={command.id}
      {...(help ? { 'data-help-topic': help.topic } : {})}
      aria-keyshortcuts={shortcut ? formatShortcut(shortcut) : undefined}
      aria-description={reason ?? summary}
      className={styles.menuItem}
      onClick={onRun}
      onKeyDown={onKeyDown}
    >
      <span className={styles.menuItemLabel}>{label}</span>
      {reason ? <span className={styles.hint}>{reason}</span> : null}
      {shortcut ? (
        <span className={styles.hint}>{formatShortcut(shortcut)}</span>
      ) : null}
    </button>
  )
}
