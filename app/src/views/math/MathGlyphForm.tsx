import { useTranslation } from 'react-i18next'
import type { DocId, MathGlyphInfo } from '../../api'
import { MathAssemblyPreview } from './MathAssemblyPreview'
import { MathConstructionForm } from './MathConstructionForm'
import { MathKernForm } from './MathKernForm'
import { OptionalMathValue } from './MathValueField'
import styles from './math.module.css'

/** Placement, kerning and construction data of one glyph record. */
export function MathGlyphForm({
  doc,
  glyph,
  info,
  overlap,
  upem,
  suggestions,
  onChange,
}: {
  doc: DocId
  glyph: string
  info: MathGlyphInfo
  overlap: number
  upem: number
  suggestions: string
  onChange: (info: MathGlyphInfo) => void
}) {
  const { t } = useTranslation()
  return (
    <div className={styles.section}>
      <OptionalMathValue
        label={t('math.italicCorrection')}
        value={info.italic_correction}
        onChange={(italic_correction) =>
          onChange({ ...info, italic_correction })
        }
      />
      <OptionalMathValue
        label={t('math.topAccent')}
        value={info.top_accent}
        onChange={(top_accent) => onChange({ ...info, top_accent })}
      />
      <label className={styles.checkRow}>
        <input
          type="checkbox"
          checked={info.extended_shape}
          onChange={(event) =>
            onChange({ ...info, extended_shape: event.target.checked })
          }
        />
        {t('math.extendedShape')}
      </label>
      <MathKernForm
        kern={info.kern}
        onChange={(kern) => onChange({ ...info, kern })}
      />
      {(['vertical', 'horizontal'] as const).map((direction) => {
        const construction = info[direction]
        return (
          <div key={direction}>
            <MathConstructionForm
              legend={t(`math.${direction}`)}
              value={construction}
              suggestions={suggestions}
              onChange={(value) => onChange({ ...info, [direction]: value })}
            />
            {construction &&
              (construction.variants.length > 0 || construction.assembly) && (
                <MathAssemblyPreview
                  doc={doc}
                  glyph={glyph}
                  vertical={direction === 'vertical'}
                  construction={construction}
                  overlap={overlap}
                  upem={upem}
                />
              )}
          </div>
        )
      })}
    </div>
  )
}
