import { useId, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ColorTable, ColrLayer, DocId } from '../../api'
import { Button } from '../../components/Button'
import { useGlyphSuggestions } from '../math/useGlyphSuggestions'
import { ColorPreview } from './ColorPreview'
import styles from './color.module.css'

/** Filterable named glyph stacks; every structural edit checks pending input. */
export function ColorGlyphsPanel({
  doc,
  table,
  palette,
  generation,
  validate,
  onChange,
}: {
  doc: DocId
  table: ColorTable
  palette: number
  generation: number
  validate: () => boolean
  onChange: (table: ColorTable) => void
}) {
  const { t } = useTranslation()
  const list = useId()
  const [query, setQuery] = useState('')
  const [adding, setAdding] = useState('')
  const [suggestionQuery, setSuggestionQuery] = useState('')
  const suggestions = useGlyphSuggestions(doc, suggestionQuery)
  const [selected, setSelected] = useState<string | null>(null)
  const names = useMemo(() => Object.keys(table.glyphs).sort(), [table.glyphs])
  const matches = names.filter((name) =>
    name.toLowerCase().includes(query.toLowerCase()),
  )
  const current = selected && table.glyphs[selected] ? selected : names[0]
  const layers = current ? table.glyphs[current] : undefined
  const setLayers = (next: ColrLayer[]) => {
    if (!current) return
    const glyphs = { ...table.glyphs }
    if (next.length) glyphs[current] = next
    else delete glyphs[current]
    onChange({ ...table, glyphs })
  }
  const add = () => {
    if (!validate() || !adding.trim()) return
    const name = adding.trim()
    if (!(name in table.glyphs))
      onChange({
        ...table,
        glyphs: {
          ...table.glyphs,
          [name]: [{ glyph: name, palette_index: 0 }],
        },
      })
    setSelected(name)
    setAdding('')
  }
  return (
    <section className={styles.section} aria-label={t('color.glyphs')}>
      <h3>{t('color.glyphs')}</h3>
      <datalist id={list}>
        {suggestions.map((name) => (
          <option key={name} value={name} />
        ))}
      </datalist>
      <div className={styles.row}>
        <input
          type="search"
          aria-label={t('color.filterGlyphs')}
          placeholder={t('color.filterGlyphs')}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <input
          aria-label={t('color.newGlyph')}
          placeholder={t('color.newGlyph')}
          value={adding}
          list={list}
          onChange={(e) => {
            setAdding(e.target.value)
            setSuggestionQuery(e.target.value)
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault()
              e.stopPropagation()
              add()
            }
          }}
        />
        <Button disabled={!adding.trim()} onClick={add}>
          {t('color.addGlyph')}
        </Button>
      </div>
      <div className={styles.workspace}>
        <div className={styles.section}>
          <select
            className={styles.records}
            size={8}
            aria-label={t('color.chooseGlyph')}
            value={current ?? ''}
            onChange={(e) => {
              if (validate()) setSelected(e.target.value)
            }}
          >
            {matches.slice(0, 2000).map((name) => (
              <option value={name} key={name}>
                {name}
              </option>
            ))}
          </select>
          <p>
            {t('color.glyphCount', {
              shown: Math.min(matches.length, 2000),
              total: names.length,
            })}
          </p>
        </div>
        <div className={styles.section}>
          {layers && current ? (
            <>
              <div className={styles.row}>
                <h4>{t('color.layers', { glyph: current })}</h4>
                <span className={styles.spacer} />
                <Button
                  onClick={() => {
                    if (validate()) setLayers([])
                  }}
                >
                  {t('color.removeGlyph')}
                </Button>
              </div>
              <p>{t('color.layerHelp')}</p>
              <ol className={styles.layers}>
                {layers.map((layer, index) => (
                  <li key={`${current}:${index}`} className={styles.layer}>
                    <span>{t('color.layer', { number: index + 1 })}</span>
                    <input
                      required
                      aria-label={t('color.outline', { number: index + 1 })}
                      data-validation-message={t('color.invalidGlyph')}
                      list={list}
                      value={layer.glyph}
                      onChange={(e) => {
                        setSuggestionQuery(e.target.value)
                        setLayers(
                          layers.map((l, i) =>
                            i === index ? { ...l, glyph: e.target.value } : l,
                          ),
                        )
                      }}
                    />
                    <select
                      aria-label={t('color.paint', { number: index + 1 })}
                      value={layer.palette_index ?? 'foreground'}
                      onChange={(e) => {
                        const palette_index =
                          e.target.value === 'foreground'
                            ? null
                            : Number(e.target.value)
                        setLayers(
                          layers.map((l, i) =>
                            i === index ? { ...l, palette_index } : l,
                          ),
                        )
                      }}
                    >
                      <option value="foreground">
                        {t('color.foreground')}
                      </option>
                      {table.palettes[palette].colors.map((_, i) => (
                        <option key={i} value={i}>
                          {t('color.entry', { number: i })}
                        </option>
                      ))}
                    </select>
                    {([-1, 1] as const).map((direction) => (
                      <Button
                        key={direction}
                        disabled={
                          index + direction < 0 ||
                          index + direction >= layers.length
                        }
                        aria-label={t(
                          direction < 0 ? 'color.moveDown' : 'color.moveUp',
                          { number: index + 1 },
                        )}
                        onClick={() => {
                          if (!validate()) return
                          const next = [...layers]
                          ;[next[index], next[index + direction]] = [
                            next[index + direction],
                            next[index],
                          ]
                          setLayers(next)
                        }}
                      >
                        {direction < 0 ? '↓' : '↑'}
                      </Button>
                    ))}
                    <Button
                      aria-label={t('color.removeLayer', { number: index + 1 })}
                      onClick={() => {
                        // Explicit removal also provides a way to discard an invalid glyph row.
                        setLayers(layers.filter((_, i) => i !== index))
                      }}
                    >
                      ×
                    </Button>
                  </li>
                ))}
              </ol>
              <Button
                disabled={layers.length >= 65535}
                onClick={() => {
                  if (validate())
                    setLayers([...layers, { glyph: current, palette_index: 0 }])
                }}
              >
                {t('color.addLayer')}
              </Button>
              <ColorPreview
                doc={doc}
                glyph={current}
                layers={layers}
                palette={table.palettes[palette].colors}
                generation={generation}
              />
            </>
          ) : (
            <p>{t('color.emptyPreview')}</p>
          )}
        </div>
      </div>
    </section>
  )
}
