import { useEffect, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { useUiStore } from '../stores/uiStore'
import { useKeymapStore } from '../stores/keymapStore'
import { shortcutFor } from './keymap'

import { reportError } from './commands'
import paletteStyles from './CommandPalette.module.css'
import dialogStyles from './dialog.module.css'
import {
  paletteSections,
  type PaletteEntry,
  type PaletteScope,
} from './paletteSearch'
import { formatShortcut } from './shortcuts'
import { usePaletteEntries } from './usePaletteEntries'

/**
 * Searchable list of every command: arrow keys move the highlight, Enter runs,
 * Escape dismisses. Commands whose preconditions are unmet stay listed, greyed
 * and annotated with the reason, so the surface is discoverable. Focus returns
 * to the element that had it when the palette opened, unless the command that
 * ran moved focus itself.
 */
export function CommandPaletteDialog() {
  const { t } = useTranslation()
  const overrides = useKeymapStore((state) => state.overrides)
  const setPaletteOpen = useUiStore((state) => state.setPaletteOpen)
  const [query, setQuery] = useState('')
  const [scope, setScope] = useState<PaletteScope>('all')
  const [active, setActive] = useState(0)
  const listId = useId()
  const dialogRef = useRef<HTMLDivElement>(null)
  const listRef = useRef<HTMLDivElement>(null)
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

  const { entries, notices, loading } = usePaletteEntries(query, scope)
  const sections = paletteSections(entries)
  // The keyboard walks the list the user can see. The ranked order decides the
  // order inside a section, but the sections decide what row two is, so the
  // highlight, Enter and every row index come from this one flattened list.
  const ordered = sections.flatMap((section) => section.entries)
  const activeIndex = Math.min(active, Math.max(0, ordered.length - 1))
  // One section needs no heading: the category selector already says which.
  const labelled = sections.length > 1

  useEffect(() => {
    const highlighted = listRef.current?.querySelector('[data-active="true"]')
    // jsdom does not implement scrollIntoView.
    if (highlighted instanceof HTMLElement && highlighted.scrollIntoView)
      highlighted.scrollIntoView({ block: 'nearest' })
  }, [activeIndex, ordered.length])

  const close = () => setPaletteOpen(false)

  const run = (command: PaletteEntry | undefined) => {
    if (!command) return
    // Disabled commands stay listed but do nothing; keep the palette open so
    // the reason next to the row remains visible.
    if (!command.enabled) return
    close()
    try {
      void Promise.resolve(command.run()).catch(reportError)
    } catch (error) {
      reportError(error)
    }
  }

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === 'Escape') {
      event.preventDefault()
      close()
    } else if (event.key === 'Tab') {
      const controls = dialogRef.current?.querySelectorAll<
        HTMLInputElement | HTMLSelectElement
      >('input, select')
      const first = controls?.[0]
      const last = controls?.[controls.length - 1]
      if (event.shiftKey && event.target === first) {
        event.preventDefault()
        last?.focus()
      } else if (!event.shiftKey && event.target === last) {
        event.preventDefault()
        first?.focus()
      }
    } else if (event.target instanceof HTMLSelectElement) {
      return
    } else if (event.key === 'ArrowDown') {
      event.preventDefault()
      setActive((index) =>
        ordered.length === 0 ? 0 : (index + 1) % ordered.length,
      )
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      setActive((index) =>
        ordered.length === 0
          ? 0
          : (index - 1 + ordered.length) % ordered.length,
      )
    } else if (event.key === 'Enter') {
      event.preventDefault()
      run(ordered[activeIndex])
    }
  }

  const activeCommand = ordered[activeIndex]

  return (
    <div
      className={dialogStyles.overlay}
      onPointerDown={(event) => {
        if (event.target === event.currentTarget) close()
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={t('palette.title')}
        className={`${dialogStyles.dialog} ${paletteStyles.palette}`}
        onKeyDown={onKeyDown}
      >
        <input
          autoFocus
          role="combobox"
          aria-expanded="true"
          aria-busy={loading}
          aria-controls={listId}
          aria-activedescendant={
            activeCommand ? `${listId}-${activeCommand.id}` : undefined
          }
          aria-label={t('palette.title')}
          className={paletteStyles.paletteInput}
          placeholder={t('paletteSearch.placeholder')}
          value={query}
          onChange={(event) => {
            setQuery(event.target.value)
            setActive(0)
          }}
        />
        <label className={paletteStyles.paletteScope}>
          <span>{t('paletteSearch.scope')}</span>
          <select
            value={scope}
            onChange={(event) => {
              setScope(event.target.value as PaletteScope)
              setActive(0)
            }}
          >
            {(['all', 'commands', 'filters', 'scripts', 'glyphs'] as const).map(
              (kind) => (
                <option key={kind} value={kind}>
                  {t(`paletteSearch.${kind}`)}
                </option>
              ),
            )}
          </select>
        </label>
        {ordered.length === 0 ? (
          <p className={paletteStyles.paletteEmpty}>
            {t(loading ? 'paletteSearch.loading' : 'paletteSearch.empty')}
          </p>
        ) : (
          <div
            ref={listRef}
            id={listId}
            role="listbox"
            aria-label={t('palette.title')}
            className={paletteStyles.paletteList}
          >
            {sections.map((section) => (
              <div
                key={section.kind}
                role="group"
                aria-label={t(`paletteSearch.${section.kind}`)}
              >
                {labelled ? (
                  <p
                    className={paletteStyles.paletteSection}
                    aria-hidden="true"
                  >
                    {t(`paletteSearch.${section.kind}`)}
                  </p>
                ) : null}
                {section.entries.map((command) => {
                  const index = ordered.indexOf(command)
                  const shortcut = shortcutFor(
                    command.id,
                    command.shortcut,
                    overrides,
                  )
                  const enabled = command.enabled
                  const reason =
                    !enabled && command.disabledReason
                      ? command.disabledReason
                      : undefined
                  // The four states are mutually exclusive so no cascade order
                  // can put disabled (faint) text on the solid accent fill.
                  const classes = [paletteStyles.paletteItem]
                  if (enabled && index === activeIndex)
                    classes.push(paletteStyles.paletteItemActive)
                  else if (!enabled && index === activeIndex)
                    classes.push(paletteStyles.paletteItemHighlight)
                  else if (!enabled)
                    classes.push(paletteStyles.paletteItemDisabled)
                  return (
                    <button
                      key={command.id}
                      type="button"
                      id={`${listId}-${command.id}`}
                      role="option"
                      aria-selected={index === activeIndex}
                      aria-disabled={!enabled}
                      aria-description={reason}
                      data-active={index === activeIndex}
                      tabIndex={-1}
                      className={classes.join(' ')}
                      onPointerEnter={() => setActive(index)}
                      onClick={() => run(command)}
                    >
                      <span className={paletteStyles.paletteLabel}>
                        {command.label}
                      </span>
                      {reason ? (
                        <span className={paletteStyles.paletteReason}>
                          {reason}
                        </span>
                      ) : null}
                      {command.detail ? (
                        <span className={paletteStyles.paletteMenu}>
                          {command.detail}
                        </span>
                      ) : null}
                      {shortcut ? (
                        <span className={paletteStyles.hint}>
                          {formatShortcut(shortcut)}
                        </span>
                      ) : null}
                    </button>
                  )
                })}
              </div>
            ))}
          </div>
        )}
        {notices.length > 0 && (
          <div role="status" className={paletteStyles.paletteNotices}>
            {notices.map((notice) => (
              <p key={notice}>{notice}</p>
            ))}
          </div>
        )}
        <div className={paletteStyles.paletteFooter}>
          {t('paletteSearch.hint')}
        </div>
      </div>
    </div>
  )
}
