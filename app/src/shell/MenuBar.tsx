import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  MENU_ORDER,
  commandsForMenu,
  runCommand,
  type MenuId,
} from './commands'
import styles from './MenuBar.module.css'
import { MenuItem } from './MenuItem'
import { useCommandRegistry } from './useCommandRegistry'

/**
 * The in-app menu bar. Every entry comes from the command registry, so a
 * command only has to declare its menu to appear here. All menus from the
 * design spec are always present; those without commands yet show a single
 * disabled placeholder rather than disappearing.
 */
export function MenuBar() {
  const { t } = useTranslation()
  useCommandRegistry()
  const [open, setOpen] = useState<MenuId | null>(null)
  const pendingItemFocus = useRef<'first' | 'last' | null>(null)
  const returnFocusRef = useRef<HTMLElement | null>(null)
  const barRef = useRef<HTMLDivElement>(null)
  const popupRef = useRef<HTMLDivElement>(null)

  const focusTitle = useCallback((menu: MenuId) => {
    barRef.current
      ?.querySelector<HTMLButtonElement>(`[data-menu="${menu}"]`)
      ?.focus()
  }, [])

  const focusItem = useCallback((edge: 'first' | 'last') => {
    const items = popupRef.current?.querySelectorAll<HTMLButtonElement>(
      '[role="menuitem"]:not(:disabled)',
    )
    ;(edge === 'first' ? items?.[0] : items?.[items.length - 1])?.focus()
  }, [])

  const rememberFocus = (element: EventTarget | null) => {
    if (
      element instanceof HTMLElement &&
      element !== document.body &&
      !barRef.current?.contains(element)
    )
      returnFocusRef.current = element
  }

  const enterMenu = (menu: MenuId, edge: 'first' | 'last') => {
    if (open === menu) focusItem(edge)
    else {
      pendingItemFocus.current = edge
      setOpen(menu)
    }
  }

  const moveMenu = useCallback(
    (current: MenuId, delta: number, keepOpen: boolean) => {
      const index = MENU_ORDER.indexOf(current)
      const next =
        MENU_ORDER[(index + delta + MENU_ORDER.length) % MENU_ORDER.length]
      if (!next) return
      if (keepOpen) setOpen(next)
      focusTitle(next)
    },
    [focusTitle],
  )

  useEffect(() => {
    if (open === null) return
    const onPointerDown = (event: PointerEvent) => {
      if (!barRef.current?.contains(event.target as Node)) setOpen(null)
    }
    document.addEventListener('pointerdown', onPointerDown)
    return () => document.removeEventListener('pointerdown', onPointerDown)
  }, [open])

  useEffect(() => {
    if (open === null || pendingItemFocus.current === null) return
    focusItem(pendingItemFocus.current)
    pendingItemFocus.current = null
  }, [open, focusItem])

  const onItemKeyDown = (event: React.KeyboardEvent, menu: MenuId) => {
    const items = [
      ...(popupRef.current?.querySelectorAll<HTMLButtonElement>(
        '[role="menuitem"]:not(:disabled)',
      ) ?? []),
    ]
    const index = items.indexOf(event.target as HTMLButtonElement)
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      const delta = event.key === 'ArrowDown' ? 1 : -1
      items[(index + delta + items.length) % items.length]?.focus()
    } else if (event.key === 'Home' || event.key === 'End') {
      event.preventDefault()
      ;(event.key === 'Home' ? items[0] : items[items.length - 1])?.focus()
    } else if (event.key === 'Escape') {
      event.preventDefault()
      setOpen(null)
      focusTitle(menu)
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault()
      pendingItemFocus.current = 'first'
      moveMenu(menu, event.key === 'ArrowRight' ? 1 : -1, true)
    } else if (event.key === 'Tab') {
      // Continue native focus traversal from this menu's title, with its
      // popup removed, instead of walking every action as a separate stop.
      setOpen(null)
      focusTitle(menu)
    }
  }

  const onTitleKeyDown = (event: React.KeyboardEvent, menu: MenuId) => {
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault()
      moveMenu(menu, event.key === 'ArrowRight' ? 1 : -1, open !== null)
    } else if (
      event.key === 'ArrowDown' ||
      event.key === 'ArrowUp' ||
      event.key === 'Enter' ||
      event.key === ' '
    ) {
      event.preventDefault()
      enterMenu(menu, event.key === 'ArrowUp' ? 'last' : 'first')
    } else if (event.key === 'Escape') {
      setOpen(null)
    }
  }

  const renderItems = (menu: MenuId) => {
    const commands = commandsForMenu(menu)
    if (commands.length === 0)
      return (
        <button
          type="button"
          role="menuitem"
          disabled
          className={styles.menuItem}
        >
          <span className={styles.menuItemLabel}>{t('menu.empty')}</span>
        </button>
      )

    const nodes: React.ReactNode[] = []
    let previousGroup: string | undefined
    for (const command of commands) {
      if (previousGroup !== undefined && command.group !== previousGroup)
        nodes.push(
          <div key={`sep-${command.id}`} className={styles.menuSeparator} />,
        )
      previousGroup = command.group
      nodes.push(
        <MenuItem
          key={command.id}
          command={command}
          onRun={() => {
            setOpen(null)
            // Restore before running: a command that opens a dialog or moves
            // to another view can then choose its own focus destination.
            if (returnFocusRef.current?.isConnected)
              returnFocusRef.current.focus()
            else focusTitle(menu)
            runCommand(command.id)
          }}
          onKeyDown={(event) => onItemKeyDown(event, menu)}
        />,
      )
    }
    return nodes
  }

  return (
    <div
      ref={barRef}
      className={styles.menubar}
      onPointerDownCapture={() => rememberFocus(document.activeElement)}
      onFocusCapture={(event) => rememberFocus(event.relatedTarget)}
      onBlurCapture={(event) => {
        if (
          !(event.relatedTarget instanceof Node) ||
          !barRef.current?.contains(event.relatedTarget)
        )
          setOpen(null)
      }}
    >
      <span className={styles.wordmark}>{t('app.name')}</span>
      <div
        role="menubar"
        aria-label={t('a11y.menubar')}
        className={styles.menuList}
      >
        {MENU_ORDER.map((menu) => (
          <div key={menu} role="none" className={styles.menuRoot}>
            <button
              type="button"
              data-menu={menu}
              role="menuitem"
              aria-haspopup="menu"
              aria-expanded={open === menu}
              className={styles.menuTitle}
              onClick={() => setOpen(open === menu ? null : menu)}
              onPointerEnter={() => open !== null && setOpen(menu)}
              onKeyDown={(event) => onTitleKeyDown(event, menu)}
            >
              {t(`menu.${menu}`)}
            </button>
            {open === menu ? (
              <div
                ref={popupRef}
                role="menu"
                aria-label={t(`menu.${menu}`)}
                className={styles.menuPopup}
              >
                {renderItems(menu)}
              </div>
            ) : null}
          </div>
        ))}
      </div>
    </div>
  )
}
