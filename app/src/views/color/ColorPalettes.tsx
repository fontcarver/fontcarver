import { graphPaletteIndices } from './graphModel'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { ColorTable, PaletteColor } from '../../api'
import { Button } from '../../components/Button'
import { ParsedInput } from '../math/ParsedInput'
import { formatColor, parseColor, removeEntry } from './colorModel'
import styles from './color.module.css'

const PAGE = 16

/** Palettes share entry indices; structural edits update every palette together. */
export function ColorPalettes({
  table,
  selected,
  onSelect,
  validate,
  onChange,
}: {
  table: ColorTable
  selected: number
  onSelect: (index: number) => void
  validate: () => boolean
  onChange: (table: ColorTable) => void
}) {
  const { t } = useTranslation()
  const [page, setPage] = useState(0)
  const palette = table.palettes[selected]
  const length = palette.colors.length
  const start = Math.min(page, Math.floor((length - 1) / PAGE)) * PAGE
  const usedEntries = useMemo(
    () =>
      new Set([
        ...Object.values(table.glyphs).flatMap((layers) =>
          layers.map((l) => l.palette_index),
        ),
        ...graphPaletteIndices(table.graph),
      ]),
    [table.glyphs, table.graph],
  )
  const setColor = (index: number, value: PaletteColor) =>
    onChange({
      ...table,
      palettes: table.palettes.map((p, i) =>
        i !== selected
          ? p
          : { ...p, colors: p.colors.map((c, j) => (j === index ? value : c)) },
      ),
    })
  return (
    <section className={styles.section} aria-label={t('color.palettes')}>
      <h3>{t('color.palettes')}</h3>
      <div className={styles.row}>
        {table.palettes.length > 256 ? (
          <ParsedInput
            label={t('color.choosePalette')}
            value={selected + 1}
            format={String}
            parse={(text) =>
              /^\d+$/.test(text) &&
              Number(text) >= 1 &&
              Number(text) <= table.palettes.length
                ? Number(text)
                : undefined
            }
            invalid={t('color.invalidPaletteIndex', {
              max: table.palettes.length,
            })}
            required
            inputMode="numeric"
            onChange={(next) => {
              if (validate()) onSelect(next - 1)
            }}
          />
        ) : (
          <select
            aria-label={t('color.choosePalette')}
            value={selected}
            onChange={(e) => {
              if (validate()) onSelect(Number(e.target.value))
            }}
          >
            {table.palettes.map((_, i) => (
              <option key={i} value={i}>
                {t('color.palette', { number: i + 1 })}
              </option>
            ))}
          </select>
        )}
        <Button
          disabled={table.palettes.length >= 65535}
          onClick={() => {
            if (!validate()) return
            onChange({
              ...table,
              palettes: [...table.palettes, structuredClone(palette)],
            })
            onSelect(table.palettes.length)
          }}
        >
          {t('color.addPalette')}
        </Button>
        <Button
          disabled={table.palettes.length < 2}
          onClick={() => {
            if (!validate()) return
            onChange({
              ...table,
              palettes: table.palettes.filter((_, i) => i !== selected),
            })
            onSelect(Math.max(0, selected - 1))
          }}
        >
          {t('color.removePalette')}
        </Button>
        <Button
          disabled={length >= 65535}
          onClick={() => {
            if (!validate()) return
            onChange({
              ...table,
              palettes: table.palettes.map((p) => ({
                ...p,
                colors: [
                  ...p.colors,
                  { red: 255, green: 255, blue: 255, alpha: 255 },
                ],
              })),
              entry_labels: table.entry_labels.length
                ? [...table.entry_labels, null]
                : [],
            })
            setPage(Math.floor(length / PAGE))
          }}
        >
          {t('color.addEntry')}
        </Button>
      </div>
      <p>{t('color.entryHelp')}</p>
      <div
        className={styles.entries}
        role="group"
        aria-label={t('color.entries')}
      >
        {palette.colors.slice(start, start + PAGE).map((color, offset) => {
          const index = start + offset
          const used = usedEntries.has(index)
          return (
            <div className={styles.entry} key={`${selected}:${index}`}>
              <span>{t('color.entry', { number: index })}</span>
              <input
                type="color"
                aria-label={t('color.entryPicker', { number: index })}
                value={formatColor(color).slice(0, 7)}
                onChange={(e) => {
                  const next = parseColor(e.target.value)
                  if (next) setColor(index, { ...next, alpha: color.alpha })
                }}
              />
              <ParsedInput
                label={t('color.entryColor', { number: index })}
                value={color}
                format={formatColor}
                parse={parseColor}
                invalid={t('color.invalidColor')}
                required
                onChange={(next) => setColor(index, next)}
              />
              <Button
                disabled={length < 2 || used}
                title={used ? t('color.entryUsed') : undefined}
                aria-label={t('color.removeEntry', { number: index })}
                onClick={() => {
                  if (validate()) onChange(removeEntry(table, index))
                }}
              >
                ×
              </Button>
            </div>
          )
        })}
      </div>
      {length > PAGE && (
        <div className={styles.row}>
          <Button
            disabled={start === 0}
            onClick={() => {
              if (validate()) setPage(start / PAGE - 1)
            }}
          >
            {t('color.previousEntries')}
          </Button>
          <span>
            {t('color.entryRange', {
              start,
              end: Math.min(start + PAGE, length) - 1,
              total: length,
            })}
          </span>
          <Button
            disabled={start + PAGE >= length}
            onClick={() => {
              if (validate()) setPage(start / PAGE + 1)
            }}
          >
            {t('color.nextEntries')}
          </Button>
        </div>
      )}
    </section>
  )
}
