import { useEffect, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { useUiStore } from '../stores/uiStore'

import { listCommands, runCommand, type Command } from './commands'
import { fuzzyRank } from './fuzzy'
import { formatShortcut } from './shortcuts'
import styles from './shell.module.css'
import { useCommandRegistry } from './useCommandRegistry'

/**
 * Searchable list of every enabled command: arrow keys move the highlight,
 * Enter runs, Escape dismisses. Focus returns to the element that had it when
 * the palette opened, unless the command that ran moved focus itself.
 */
export function CommandPaletteDialog() {
  const { t } = useTranslation()
  useCommandRegistry()
  const setPaletteOpen = useUiStore((state) => state.setPaletteOpen)
  const [query, setQuery] = useState('')
  const [active, setActive] = useState(0)
  const listId = useId()
  const dialogRef = useRef<HTMLDivElement>(null)
  const listRef = useRef<HTMLUListElement>(null)
  const [restoreFocus] = useState<HTMLElement | null>(() =>
    document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null,
  )

  useEffect(() => {
    const dialog = dialogRef.current
    return () => {
      const focused = document.activeElement
      const focusLeftThePalette =
        focused !== null &&
        focused !== document.body &&
        !dialog?.contains(focused)
      if (!focusLeftThePalette) restoreFocus?.focus()
    }
  }, [restoreFocus])

  // Recomputed on every render: the list is small, `when()` must be fresh, and
  // `useCommandRegistry()` above re-renders us when the registry changes.
  const enabled = listCommands().filter(
    (command) => command.id !== 'palette.open' && (command.when?.() ?? true),
  )
  const commands = fuzzyRank(query, enabled, (command) => t(command.title))

  useEffect(() => {
    const highlighted = listRef.current?.querySelector('[data-active="true"]')
    // jsdom does not implement scrollIntoView.
    if (highlighted instanceof HTMLElement && highlighted.scrollIntoView)
      highlighted.scrollIntoView({ block: 'nearest' })
  }, [active])

  const close = () => setPaletteOpen(false)

  const run = (command: Command | undefined) => {
    if (!command) return
    close()
    runCommand(command.id)
  }

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === 'Escape') {
      event.preventDefault()
      close()
    } else if (event.key === 'ArrowDown') {
      event.preventDefault()
      setActive((index) =>
        commands.length === 0 ? 0 : (index + 1) % commands.length,
      )
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setActive((index) =>
        commands.length === 0
          ? 0
          : (index - 1 + commands.length) % commands.length,
      )
    } else if (event.key === 'Enter') {
      event.preventDefault()
      run(commands[active])
    }
  }

  const activeCommand = commands[active]

  return (
    <div
      className={styles.overlay}
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) close()
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={t('palette.title')}
        className={`${styles.dialog} ${styles.palette}`}
        onKeyDown={onKeyDown}
      >
        <input
          autoFocus
          role="combobox"
          aria-expanded="true"
          aria-controls={listId}
          aria-activedescendant={
            activeCommand ? `${listId}-${activeCommand.id}` : undefined
          }
          aria-label={t('palette.title')}
          className={styles.paletteInput}
          placeholder={t('palette.placeholder')}
          value={query}
          onChange={(event) => {
            setQuery(event.target.value)
            setActive(0)
          }}
        />
        {commands.length === 0 ? (
          <p className={styles.paletteEmpty}>{t('palette.empty')}</p>
        ) : (
          <ul
            ref={listRef}
            id={listId}
            role="listbox"
            aria-label={t('palette.title')}
            className={styles.paletteList}
          >
            {commands.map((command, index) => (
              <li key={command.id}>
                <button
                  type="button"
                  id={`${listId}-${command.id}`}
                  role="option"
                  aria-selected={index === active}
                  data-active={index === active}
                  tabIndex={-1}
                  className={
                    index === active
                      ? `${styles.paletteItem} ${styles.paletteItemActive}`
                      : styles.paletteItem
                  }
                  onPointerEnter={() => setActive(index)}
                  onClick={() => run(command)}
                >
                  <span className={styles.paletteLabel}>
                    {t(command.title)}
                  </span>
                  {command.menu ? (
                    <span className={styles.paletteMenu}>
                      {t(`menu.${command.menu}`)}
                    </span>
                  ) : null}
                  {command.shortcut ? (
                    <span className={styles.shortcut}>
                      {formatShortcut(command.shortcut)}
                    </span>
                  ) : null}
                </button>
              </li>
            ))}
          </ul>
        )}
        <div className={styles.paletteFooter}>{t('palette.hint')}</div>
      </div>
    </div>
  )
}
