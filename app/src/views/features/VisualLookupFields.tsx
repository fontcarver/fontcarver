import { useTranslation } from 'react-i18next'
import type { VisualLookup } from './visualLookup'
import styles from './VisualLookupEditor.module.css'

type Value = {
  xPlacement: number
  yPlacement: number
  xAdvance: number
  yAdvance: number
}
type Anchor = { x: number; y: number }

/** Typed controls render the operands of one supported lookup statement. */
export function VisualLookupFields({
  rule,
  disabled,
  onChange,
}: {
  rule: VisualLookup
  disabled: boolean
  onChange: (rule: VisualLookup) => void
}) {
  const { t } = useTranslation()
  const text = (
    label: string,
    value: string,
    change: (value: string) => void,
  ) => (
    <label className={styles.field}>
      {t(`visualLookup.${label}`)}
      <input
        value={value}
        disabled={disabled}
        spellCheck={false}
        onChange={(event) => change(event.target.value)}
      />
    </label>
  )
  const numeric = (
    label: string,
    value: number,
    change: (value: number) => void,
  ) => (
    <label className={styles.field}>
      {t(`visualLookup.${label}`)}
      <input
        type="number"
        step="1"
        min="-32768"
        max="32767"
        value={Number.isFinite(value) ? value : ''}
        disabled={disabled}
        onChange={(event) => change(event.target.valueAsNumber)}
      />
    </label>
  )
  const value = (
    label: string,
    record: Value,
    change: (next: Value) => void,
  ) => (
    <fieldset className={styles.values}>
      <legend>{t(`visualLookup.${label}`)}</legend>
      {numeric('xPlacement', record.xPlacement, (xPlacement) =>
        change({ ...record, xPlacement }),
      )}
      {numeric('yPlacement', record.yPlacement, (yPlacement) =>
        change({ ...record, yPlacement }),
      )}
      {numeric('xAdvance', record.xAdvance, (xAdvance) =>
        change({ ...record, xAdvance }),
      )}
      {numeric('yAdvance', record.yAdvance, (yAdvance) =>
        change({ ...record, yAdvance }),
      )}
    </fieldset>
  )
  const anchor = (
    label: string,
    record: Anchor,
    change: (next: Anchor) => void,
  ) => (
    <fieldset className={styles.values}>
      <legend>{t(`visualLookup.${label}`)}</legend>
      {numeric('x', record.x, (x) => change({ ...record, x }))}
      {numeric('y', record.y, (y) => change({ ...record, y }))}
    </fieldset>
  )
  switch (rule.kind) {
    case 'gsub-single':
    case 'gsub-multiple':
    case 'gsub-alternate':
    case 'gsub-ligature':
      return (
        <div className={styles.operands}>
          {text('input', rule.from, (from) => onChange({ ...rule, from }))}
          {text('output', rule.to, (to) => onChange({ ...rule, to }))}
        </div>
      )
    case 'gsub-contextual':
      return (
        <div className={styles.operands}>
          {text('before', rule.before, (before) =>
            onChange({ ...rule, before }),
          )}
          {text('input', rule.input, (input) => onChange({ ...rule, input }))}
          {text('after', rule.after, (after) => onChange({ ...rule, after }))}
          {text('output', rule.replacement, (replacement) =>
            onChange({ ...rule, replacement }),
          )}
        </div>
      )
    case 'gpos-single':
      return (
        <div className={styles.operands}>
          {text('input', rule.glyphs, (glyphs) =>
            onChange({ ...rule, glyphs }),
          )}
          {value('adjustment', rule.value, (next) =>
            onChange({ ...rule, value: next }),
          )}
        </div>
      )
    case 'gpos-pair':
      return (
        <div className={styles.operands}>
          {text('firstGlyphs', rule.first, (first) =>
            onChange({ ...rule, first }),
          )}
          {text('secondGlyphs', rule.second, (second) =>
            onChange({ ...rule, second }),
          )}
          {value('firstAdjustment', rule.firstValue, (firstValue) =>
            onChange({ ...rule, firstValue }),
          )}
          {value('secondAdjustment', rule.secondValue, (secondValue) =>
            onChange({ ...rule, secondValue }),
          )}
        </div>
      )
    case 'gpos-mark-base':
    case 'gpos-mark-mark':
      return (
        <div className={styles.operands}>
          {text('markGlyphs', rule.markGlyphs, (markGlyphs) =>
            onChange({ ...rule, markGlyphs }),
          )}
          {text('targetGlyphs', rule.targetGlyphs, (targetGlyphs) =>
            onChange({ ...rule, targetGlyphs }),
          )}
          {text('markClass', rule.markClass, (markClass) =>
            onChange({ ...rule, markClass }),
          )}
          {anchor('markAnchor', rule.markAnchor, (markAnchor) =>
            onChange({ ...rule, markAnchor }),
          )}
          {anchor('targetAnchor', rule.targetAnchor, (targetAnchor) =>
            onChange({ ...rule, targetAnchor }),
          )}
        </div>
      )
  }
}
