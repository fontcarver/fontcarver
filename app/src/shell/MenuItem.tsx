import { useTranslation } from 'react-i18next'

import type { Command } from './commands'
import styles from './MenuBar.module.css'
import { formatShortcut } from './shortcuts'
import { shortcutFor } from './keymap'
import { useKeymapStore } from '../stores/keymapStore'
import { helpHint } from './contextHelp'

export interface MenuItemProps {
  command: Command
  onRun: () => void
  onKeyDown: (event: React.KeyboardEvent) => void
}

/**
 * One row of a menu popup. Disabled commands stay listed and explain why they
 * are unavailable through their tooltip and accessible description.
 */
export function MenuItem({ command, onRun, onKeyDown }: MenuItemProps) {
  const { t } = useTranslation()
  const overrides = useKeymapStore((state) => state.overrides)
  const shortcut = shortcutFor(command.id, command.shortcut, overrides)
  const hint = helpHint(
    command.menu === 'glyph' ||
      command.menu === 'path' ||
      command.menu === 'layer'
      ? 'glyph'
      : command.menu === 'font'
        ? 'font'
        : command.menu === 'file'
          ? 'files'
          : 'shortcuts',
  )
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
      title={
        reason ??
        `${label}${shortcut ? ` (${formatShortcut(shortcut)})` : ''}. ${hint.title}`
      }
      data-help-topic={hint['data-help-topic']}
      aria-description={reason ?? hint['aria-description']}
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
