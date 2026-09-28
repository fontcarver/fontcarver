import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ColorTable, DocId } from '../../api'
import { Button } from '../../components/Button'
import { addGradient, parseGraph } from './graphModel'
import { ColorGraphPreview } from './ColorGraphPreview'
import styles from './color.module.css'

/** Paint graph source stays in the form, including invalid text during handovers. */
export function ColorGraphPanel({
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
  const graph = table.graph
  const [draft, setDraft] = useState({
    from: graph,
    text: JSON.stringify(graph, null, 2) ?? '',
    invalid: false,
  })
  const [glyph, setGlyph] = useState('')
  const [previewGlyph, setPreviewGlyph] = useState('')
  const input = useRef<HTMLTextAreaElement>(null)
  if (draft.from !== graph)
    setDraft({
      from: graph,
      text: JSON.stringify(graph, null, 2) ?? '',
      invalid: false,
    })
  useEffect(() => {
    input.current?.setCustomValidity(
      draft.invalid ? t('color.graphInvalid') : '',
    )
  }, [draft.invalid, t])
  const names = Object.keys(graph?.glyphs ?? {})
  const current = names.includes(previewGlyph) ? previewGlyph : (names[0] ?? '')
  return (
    <section className={styles.section} aria-label={t('color.graphTitle')}>
      <h3>{t('color.graphTitle')}</h3>
      <p>{t('color.graphHelp')}</p>
      <div className={styles.row}>
        <input
          aria-label={t('color.graphNewGlyph')}
          placeholder={t('color.graphNewGlyph')}
          value={glyph}
          onChange={(e) => setGlyph(e.target.value)}
        />
        <Button
          disabled={!glyph.trim()}
          onClick={() => {
            if (!validate()) return
            onChange({
              ...table,
              graph: addGradient(
                graph,
                glyph.trim(),
                table.palettes[0].colors.length,
              ),
            })
            setPreviewGlyph(glyph.trim())
          }}
        >
          {t('color.graphAdd')}
        </Button>
        {graph && (
          <Button
            onClick={() => {
              if (validate()) onChange({ ...table, graph: null })
            }}
          >
            {t('color.graphDelete')}
          </Button>
        )}
      </div>
      {graph && (
        <>
          <label htmlFor="color-graph-source">{t('color.graphSource')}</label>
          <textarea
            id="color-graph-source"
            ref={input}
            className={styles.graphSource}
            value={draft.text}
            spellCheck={false}
            aria-invalid={draft.invalid}
            data-validation-message={t('color.graphInvalid')}
            onChange={(e) => {
              const text = e.target.value
              try {
                const next = parseGraph(text)
                e.target.setCustomValidity('')
                setDraft({ from: next, text, invalid: false })
                onChange({ ...table, graph: next })
              } catch {
                e.target.setCustomValidity(t('color.graphInvalid'))
                setDraft({ from: graph, text, invalid: true })
                onChange(table)
              }
            }}
          />
          {draft.invalid && <p role="alert">{t('color.graphInvalid')}</p>}
          <p>
            {t('color.graphCounts', {
              roots: names.length,
              paints: graph.paints.length,
            })}
          </p>
          <label className={styles.row}>
            {t('color.graphPreviewGlyph')}
            <input
              value={previewGlyph}
              placeholder={names[0] ?? ''}
              onChange={(e) => setPreviewGlyph(e.target.value)}
            />
          </label>
          {!draft.invalid && current && (
            <ColorGraphPreview
              doc={doc}
              glyph={previewGlyph || current}
              color={table}
              palette={palette}
              generation={generation}
            />
          )}
        </>
      )}
    </section>
  )
}
