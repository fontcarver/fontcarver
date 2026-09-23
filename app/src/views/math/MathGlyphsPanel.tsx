import { useId, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { DocId, MathTable } from '../../api'
import { Button } from '../../components/Button'
import { emptyGlyph } from './mathModel'
import { MathGlyphForm } from './MathGlyphForm'
import { GlyphQueryContext, useGlyphSuggestions } from './useGlyphSuggestions'
import styles from './math.module.css'

const VISIBLE_LIMIT = 2000

/** Filterable glyph records with one record editor. */
export function MathGlyphsPanel({
  doc,
  table,
  upem,
  onChange,
}: {
  doc: DocId
  table: MathTable
  upem: number
  onChange: (table: MathTable) => void
}) {
  const { t } = useTranslation()
  const suggestions = useId()
  const [query, setQuery] = useState('')
  const [adding, setAdding] = useState('')
  const [selected, setSelected] = useState<string | null>(null)
  const names = useMemo(() => Object.keys(table.glyphs).sort(), [table.glyphs])
  const matches = useMemo(() => {
    const needle = query.trim().toLowerCase()
    return needle
      ? names.filter((n) => n.toLowerCase().includes(needle))
      : names
  }, [names, query])
  const [suggestQuery, setSuggestQuery] = useState('')
  const hint = useGlyphSuggestions(doc, suggestQuery)
  const current = selected !== null ? table.glyphs[selected] : undefined
  const add = () => {
    const name = adding.trim()
    if (!name) return
    if (!(name in table.glyphs))
      onChange({ ...table, glyphs: { ...table.glyphs, [name]: emptyGlyph() } })
    setSelected(name)
    setAdding('')
  }
  return (
    <GlyphQueryContext.Provider value={setSuggestQuery}>
      <div className={styles.section}>
        <datalist id={suggestions}>
          {hint.map((name) => (
            <option key={name} value={name} />
          ))}
        </datalist>
        <div className={styles.row}>
          <input
            type="search"
            aria-label={t('math.filterRecords')}
            placeholder={t('math.filterRecords')}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <input
            aria-label={t('math.newRecord')}
            placeholder={t('math.newRecord')}
            list={suggestions}
            value={adding}
            onChange={(event) => {
              setAdding(event.target.value)
              setSuggestQuery(event.target.value)
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault()
                event.stopPropagation()
                add()
              }
            }}
          />
          <Button disabled={adding.trim() === ''} onClick={add}>
            {t('math.addRecord')}
          </Button>
        </div>
        <p>
          {t('math.recordCount', {
            shown: Math.min(matches.length, VISIBLE_LIMIT),
            total: names.length,
          })}
        </p>
        <select
          className={styles.records}
          size={8}
          aria-label={t('math.records')}
          value={selected ?? ''}
          onChange={(event) => setSelected(event.target.value)}
        >
          {matches.slice(0, VISIBLE_LIMIT).map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
        {selected !== null && current && (
          <section aria-label={t('math.recordFor', { glyph: selected })}>
            <div className={styles.row}>
              <h3>{selected}</h3>
              <Button
                onClick={() => {
                  const glyphs = { ...table.glyphs }
                  delete glyphs[selected]
                  onChange({ ...table, glyphs })
                  setSelected(null)
                }}
              >
                {t('math.removeRecord')}
              </Button>
            </div>
            <MathGlyphForm
              doc={doc}
              glyph={selected}
              info={current}
              overlap={table.min_connector_overlap}
              upem={upem}
              suggestions={suggestions}
              onChange={(info) =>
                onChange({
                  ...table,
                  glyphs: { ...table.glyphs, [selected]: info },
                })
              }
            />
          </section>
        )}
      </div>
    </GlyphQueryContext.Provider>
  )
}
