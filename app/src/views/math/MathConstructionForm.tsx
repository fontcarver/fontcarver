import { useContext } from 'react'
import { useTranslation } from 'react-i18next'
import type { MathAssembly, MathConstruction, MathPart } from '../../api'
import { Button } from '../../components/Button'
import { MathValueField } from './MathValueField'
import { ParsedInput, parseUnsigned } from './ParsedInput'
import { GlyphQueryContext } from './useGlyphSuggestions'
import styles from './math.module.css'

const glyphName = (text: string) =>
  text.trim() === '' ? undefined : text.trim()

function move<T>(items: T[], index: number, delta: number): T[] {
  const target = index + delta
  if (target < 0 || target >= items.length) return items
  const next = [...items]
  const [item] = next.splice(index, 1)
  next.splice(target, 0, item!)
  return next
}

/** Ready-made size variants and an optional assembly for one direction. */
export function MathConstructionForm({
  legend,
  value,
  suggestions,
  onChange,
}: {
  legend: string
  value: MathConstruction | null
  /** Datalist id with glyph name suggestions. */
  suggestions: string
  onChange: (value: MathConstruction | null) => void
}) {
  const { t } = useTranslation()
  const suggest = useContext(GlyphQueryContext)
  return (
    <fieldset className={styles.card}>
      <legend>
        <label>
          <input
            type="checkbox"
            checked={value !== null}
            onChange={(event) =>
              onChange(
                event.target.checked ? { variants: [], assembly: null } : null,
              )
            }
          />
          {legend}
        </label>
      </legend>
      {value && (
        <>
          <h4>{t('math.variants')}</h4>
          <p>{t('math.variantsHelp')}</p>
          {value.variants.map((variant, index) => (
            <div key={index} className={styles.itemRow}>
              <ParsedInput<string>
                label={t('math.variantGlyph', { index: index + 1 })}
                value={variant.glyph}
                format={String}
                parse={glyphName}
                invalid={t('math.invalidGlyph')}
                list={suggestions}
                required
                onText={suggest}
                onChange={(glyph) =>
                  onChange({
                    ...value,
                    variants: value.variants.map((v, i) =>
                      i === index ? { ...v, glyph } : v,
                    ),
                  })
                }
              />
              <ParsedInput<number>
                label={t('math.variantAdvance', { index: index + 1 })}
                value={variant.advance}
                format={String}
                parse={parseUnsigned}
                invalid={t('math.invalidUnsigned')}
                inputMode="decimal"
                onChange={(advance) =>
                  onChange({
                    ...value,
                    variants: value.variants.map((v, i) =>
                      i === index ? { ...v, advance } : v,
                    ),
                  })
                }
              />
              <RowButtons
                index={index}
                count={value.variants.length}
                onMove={(delta) =>
                  onChange({
                    ...value,
                    variants: move(value.variants, index, delta),
                  })
                }
                onRemove={() =>
                  onChange({
                    ...value,
                    variants: value.variants.filter((_, i) => i !== index),
                  })
                }
              />
            </div>
          ))}
          <Button
            onClick={() =>
              onChange({
                ...value,
                variants: [...value.variants, { glyph: '', advance: 0 }],
              })
            }
          >
            {t('math.addVariant')}
          </Button>
          <label className={styles.checkRow}>
            <input
              type="checkbox"
              checked={value.assembly !== null}
              onChange={(event) =>
                onChange({
                  ...value,
                  assembly: event.target.checked
                    ? {
                        italic_correction: { value: 0, device: null },
                        parts: [],
                      }
                    : null,
                })
              }
            />
            {t('math.hasAssembly')}
          </label>
          {value.assembly && (
            <AssemblyForm
              assembly={value.assembly}
              suggestions={suggestions}
              onChange={(assembly) => onChange({ ...value, assembly })}
            />
          )}
        </>
      )}
    </fieldset>
  )
}

function AssemblyForm({
  assembly,
  suggestions,
  onChange,
}: {
  assembly: MathAssembly
  suggestions: string
  onChange: (assembly: MathAssembly) => void
}) {
  const { t } = useTranslation()
  const suggest = useContext(GlyphQueryContext)
  const setPart = (index: number, patch: Partial<MathPart>) =>
    onChange({
      ...assembly,
      parts: assembly.parts.map((p, i) =>
        i === index ? { ...p, ...patch } : p,
      ),
    })
  const numeric = ['start_connector', 'end_connector', 'full_advance'] as const
  return (
    <div className={styles.assembly}>
      <p>{t('math.assemblyHelp')}</p>
      <MathValueField
        label={t('math.assemblyItalic')}
        value={assembly.italic_correction}
        onChange={(italic_correction) =>
          onChange({ ...assembly, italic_correction })
        }
      />
      {assembly.parts.map((part, index) => (
        <div key={index} className={styles.itemRow}>
          <ParsedInput<string>
            label={t('math.partGlyph', { index: index + 1 })}
            value={part.glyph}
            format={String}
            parse={glyphName}
            invalid={t('math.invalidGlyph')}
            list={suggestions}
            required
            onText={suggest}
            onChange={(glyph) => setPart(index, { glyph })}
          />
          {numeric.map((field) => (
            <ParsedInput<number>
              key={field}
              label={t(`math.part.${field}`, { index: index + 1 })}
              placeholder={t(`math.partShort.${field}`)}
              value={part[field]}
              format={String}
              parse={parseUnsigned}
              invalid={t('math.invalidUnsigned')}
              inputMode="decimal"
              onChange={(next) => setPart(index, { [field]: next })}
            />
          ))}
          <label>
            <input
              type="checkbox"
              checked={part.extender}
              onChange={(event) =>
                setPart(index, { extender: event.target.checked })
              }
            />
            {t('math.extender')}
          </label>
          <RowButtons
            index={index}
            count={assembly.parts.length}
            onMove={(delta) =>
              onChange({
                ...assembly,
                parts: move(assembly.parts, index, delta),
              })
            }
            onRemove={() =>
              onChange({
                ...assembly,
                parts: assembly.parts.filter((_, i) => i !== index),
              })
            }
          />
        </div>
      ))}
      <Button
        onClick={() =>
          onChange({
            ...assembly,
            parts: [
              ...assembly.parts,
              {
                glyph: '',
                start_connector: 0,
                end_connector: 0,
                full_advance: 0,
                extender: false,
              },
            ],
          })
        }
      >
        {t('math.addPart')}
      </Button>
    </div>
  )
}

function RowButtons({
  index,
  count,
  onMove,
  onRemove,
}: {
  index: number
  count: number
  onMove: (delta: number) => void
  onRemove: () => void
}) {
  const { t } = useTranslation()
  return (
    <span className={styles.rowButtons}>
      <Button disabled={index === 0} onClick={() => onMove(-1)}>
        {t('math.moveUp')}
      </Button>
      <Button disabled={index === count - 1} onClick={() => onMove(1)}>
        {t('math.moveDown')}
      </Button>
      <Button onClick={onRemove}>{t('math.remove')}</Button>
    </span>
  )
}
