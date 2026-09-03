import {
  useEffect,
  useRef,
  type KeyboardEvent as ReactKeyboardEvent,
} from 'react'
import { useTranslation } from 'react-i18next'

import { getCommand, isCommandEnabled, runCommand } from '../../shell/commands'

import styles from './fontView.module.css'

export interface GlyphContextMenuProps {
  /** Viewport coordinates of the click that opened the menu. */
  x: number
  y: number
  /** Command ids listed in the menu, in order. */
  commands: readonly string[]
  /** Closes the menu; the grid takes keyboard focus back. */
  onClose: () => void
}

/**
 * The glyph-grid context menu. It lists registry commands, so a command that
 * gains a new gate or title is reflected here without a change. The first
 * enabled item takes focus when the menu opens, and focus returns to the grid
 * when it closes.
 */
export function GlyphContextMenu({
  x,
  y,
  commands,
  onClose,
}: GlyphContextMenuProps) {
  const { t } = useTranslation()
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    ref.current
      ?.querySelector<HTMLButtonElement>(
        'button[role="menuitem"]:not(:disabled)',
      )
      ?.focus()
    const onPointerDown = (event: Event) => {
      if (!ref.current?.contains(event.target as Node)) onClose()
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    document.addEventListener('pointerdown', onPointerDown, true)
    document.addEventListener('keydown', onKeyDown, true)
    return () => {
      document.removeEventListener('pointerdown', onPointerDown, true)
      document.removeEventListener('keydown', onKeyDown, true)
    }
  }, [onClose])

  const onMenuKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
    event.preventDefault()
    const items = [
      ...(ref.current?.querySelectorAll<HTMLButtonElement>(
        'button[role="menuitem"]:not(:disabled)',
      ) ?? []),
    ]
    if (items.length === 0) return
    const current = items.indexOf(document.activeElement as HTMLButtonElement)
    const delta = event.key === 'ArrowDown' ? 1 : -1
    const next =
      (((current < 0 ? 0 : current + delta) % items.length) + items.length) %
      items.length
    items[next]?.focus()
  }

  return (
    <div
      ref={ref}
      role="menu"
      aria-label={t('fontView.contextMenu')}
      className={styles.menu}
      style={{ left: x, top: y }}
      onKeyDown={onMenuKeyDown}
    >
      {commands.map((id) => {
        const command = getCommand(id)
        if (!command) return null
        const enabled = isCommandEnabled(id)
        return (
          <button
            key={id}
            type="button"
            role="menuitem"
            className={styles.menuItem}
            disabled={!enabled}
            onClick={() => {
              onClose()
              runCommand(id)
            }}
          >
            {t(command.title)}
          </button>
        )
      })}
    </div>
  )
}
