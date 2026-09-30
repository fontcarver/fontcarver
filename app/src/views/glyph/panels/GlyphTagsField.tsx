import { useEffect, useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { DocId } from '../../../api'
import { selectDocView, useDocumentStore } from '../../../stores/documentStore'

import { completeTag, parseTags, tagSuggestions } from './glyphTags'
import styles from './glyphMetadata.module.css'
import panelStyles from './panels.module.css'

interface GlyphTagsFieldProps {
  doc: DocId
  tags: readonly string[]
  onCommit: (tags: string[]) => void
}

/** Free-text tags with keyboard completion from the document's existing tags. */
export function GlyphTagsField({ doc, tags, onCommit }: GlyphTagsFieldProps) {
  const { t } = useTranslation()
  const listId = useId()
  const input = useRef<HTMLInputElement>(null)
  const view = useDocumentStore(selectDocView(doc))
  const loadSidebar = useDocumentStore((state) => state.loadSidebar)
  const value = tags.join(', ')
  const [shown, setShown] = useState(value)
  const [draft, setDraft] = useState(value)
  const [open, setOpen] = useState(false)
  const [active, setActive] = useState(-1)
  const cancelBlur = useRef(false)
  if (shown !== value) {
    setShown(value)
    setDraft(value)
  }
  useEffect(() => {
    if (view.sidebarGeneration !== view.generation) void loadSidebar(doc)
  }, [doc, loadSidebar, view.generation, view.sidebarGeneration])
  const suggestions = tagSuggestions(draft, view.sidebar?.tags ?? [])
  const expanded = open && suggestions.length > 0
  const commit = (next: string[]) => {
    setDraft(next.join(', '))
    if (next.join(', ') !== value) onCommit(next)
  }
  const choose = (tag: string) => {
    commit(completeTag(draft, tag))
    setOpen(false)
    setActive(-1)
    input.current?.focus()
  }
  return (
    <div className={panelStyles.row}>
      <label className={panelStyles.rowLabel} htmlFor={`${listId}-input`}>
        {t('glyphInspector.tags')}
      </label>
      <div className={styles.tagField}>
        <input
          id={`${listId}-input`}
          ref={input}
          role="combobox"
          aria-autocomplete="list"
          aria-expanded={expanded}
          aria-controls={expanded ? listId : undefined}
          aria-activedescendant={
            expanded && active >= 0 ? `${listId}-${active}` : undefined
          }
          className={panelStyles.textInput}
          value={draft}
          placeholder={t('panel.glyph.tagsPlaceholder')}
          onFocus={() => setOpen(true)}
          onChange={(event) => {
            setDraft(event.currentTarget.value)
            setActive(-1)
            setOpen(true)
          }}
          onBlur={() => {
            setOpen(false)
            if (cancelBlur.current) cancelBlur.current = false
            else commit(parseTags(draft))
          }}
          onKeyDown={(event) => {
            if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
              event.preventDefault()
              setOpen(true)
              setActive((current) =>
                suggestions.length === 0
                  ? -1
                  : (current +
                      (event.key === 'ArrowDown' ? 1 : -1) +
                      suggestions.length) %
                    suggestions.length,
              )
            } else if (event.key === 'Enter') {
              event.preventDefault()
              const suggestion = expanded ? suggestions[active] : undefined
              if (suggestion) choose(suggestion)
              else {
                commit(parseTags(draft))
                setOpen(false)
              }
            } else if (event.key === 'Escape') {
              event.preventDefault()
              setDraft(value)
              setOpen(false)
              cancelBlur.current = true
              input.current?.blur()
            }
          }}
        />
        {expanded && (
          <div
            id={listId}
            role="listbox"
            aria-label={t('glyphMetadata.tagSuggestions')}
            className={styles.suggestions}
          >
            {suggestions.map((tag, index) => (
              <button
                key={tag}
                id={`${listId}-${index}`}
                type="button"
                role="option"
                aria-selected={index === active}
                onPointerDown={(event) => event.preventDefault()}
                onClick={() => choose(tag)}
              >
                {tag}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
