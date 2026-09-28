import type { StyleEffectOptions } from '../../api'

export const STYLE_PRESETS = [
  'weight',
  'lighten',
  'condense',
  'extend',
  'oblique',
  'smallCaps',
  'subscript',
  'superscript',
] as const
export type StylePreset = (typeof STYLE_PRESETS)[number]
export type StyleFields = Record<
  'scaleX' | 'scaleY' | 'moveX' | 'moveY' | 'oblique' | 'weightX' | 'weightY',
  string
>

/** Starting values only; all presets remain editable in font units. */
export function presetFields(preset: StylePreset, upem: number): StyleFields {
  const fields = {
    scaleX: '100',
    scaleY: '100',
    moveX: '0',
    moveY: '0',
    oblique: '0',
    weightX: '0',
    weightY: '0',
  }
  switch (preset) {
    case 'weight':
      fields.weightX = fields.weightY = String(Math.min(500, upem / 50))
      break
    case 'lighten':
      fields.weightX = fields.weightY = String(-Math.min(500, upem / 50))
      break
    case 'condense':
      fields.scaleX = '85'
      break
    case 'extend':
      fields.scaleX = '115'
      break
    case 'oblique':
      fields.oblique = '12'
      break
    case 'smallCaps':
      fields.scaleX = fields.scaleY = '75'
      break
    case 'subscript':
      fields.scaleX = fields.scaleY = '60'
      fields.moveY = String(-upem * 0.15)
      break
    case 'superscript':
      fields.scaleX = fields.scaleY = '60'
      fields.moveY = String(upem * 0.35)
      break
  }
  return fields
}

/** Reject blank/incomplete numeric text before constructing an API request. */
export function styleOptions(fields: StyleFields): StyleEffectOptions | null {
  const value = Object.fromEntries(
    Object.entries(fields).map(([key, text]) => [
      key,
      text.trim() === '' ? NaN : Number(text),
    ]),
  ) as Record<keyof StyleFields, number>
  if (Object.values(value).some((number) => !Number.isFinite(number)))
    return null
  return {
    scale: [value.scaleX / 100, value.scaleY / 100],
    translate: [value.moveX, value.moveY],
    oblique: value.oblique,
    weight: [value.weightX, value.weightY],
  }
}
