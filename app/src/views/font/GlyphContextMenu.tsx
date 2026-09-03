import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { getCommand, isCommandEnabled, runCommand } from '../../shell/commands'

import styles from './fontView.module.css'

export interface GlyphContextMenuProps {
  /** Viewport coordinates of the click that opened the menu. */
  x: number
  y: number
  /** Command ids listed in the menu, in order. */
  commands: readonly string[]
  onClose: () => void
}

/**
 * The glyph-grid context menu. It lists registry commands, so a command that
 * gains a new gate or title is reflected here without a change.
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
    ref.current?.focus()
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

  return (
    <div
      ref={ref}
      role="menu"
      aria-label={t('fontView.contextMenu')}
      tabIndex={-1}
      className={styles.menu}
      style={{ left: x, top: y }}
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
