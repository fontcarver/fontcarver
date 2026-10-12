import type {
  MathCommand,
  MathConstants,
  MathDevice,
  MathGlyphInfo,
  MathState,
  MathTable,
  MathValue,
} from '../../api'

/** Constants measured in font units, grouped as in the OpenType specification. */
export const CONSTANT_GROUPS = {
  general: [
    'math_leading',
    'axis_height',
    'accent_base_height',
    'flattened_accent_base_height',
  ],
  scripts: [
    'subscript_shift_down',
    'subscript_top_max',
    'subscript_baseline_drop_min',
    'superscript_shift_up',
    'superscript_shift_up_cramped',
    'superscript_bottom_min',
    'superscript_baseline_drop_max',
    'sub_superscript_gap_min',
    'superscript_bottom_max_with_subscript',
    'space_after_script',
  ],
  limits: [
    'upper_limit_gap_min',
    'upper_limit_baseline_rise_min',
    'lower_limit_gap_min',
    'lower_limit_baseline_drop_min',
  ],
  stacks: [
    'stack_top_shift_up',
    'stack_top_display_style_shift_up',
    'stack_bottom_shift_down',
    'stack_bottom_display_style_shift_down',
    'stack_gap_min',
    'stack_display_style_gap_min',
    'stretch_stack_top_shift_up',
    'stretch_stack_bottom_shift_down',
    'stretch_stack_gap_above_min',
    'stretch_stack_gap_below_min',
  ],
  fractions: [
    'fraction_numerator_shift_up',
    'fraction_numerator_display_style_shift_up',
    'fraction_denominator_shift_down',
    'fraction_denominator_display_style_shift_down',
    'fraction_numerator_gap_min',
    'fraction_num_display_style_gap_min',
    'fraction_rule_thickness',
    'fraction_denominator_gap_min',
    'fraction_denom_display_style_gap_min',
    'skewed_fraction_horizontal_gap',
    'skewed_fraction_vertical_gap',
  ],
  bars: [
    'overbar_vertical_gap',
    'overbar_rule_thickness',
    'overbar_extra_ascender',
    'underbar_vertical_gap',
    'underbar_rule_thickness',
    'underbar_extra_descender',
  ],
  radicals: [
    'radical_vertical_gap',
    'radical_display_style_vertical_gap',
    'radical_rule_thickness',
    'radical_extra_ascender',
    'radical_kern_before_degree',
    'radical_kern_after_degree',
  ],
} as const satisfies Record<string, readonly (keyof MathConstants)[]>

/** One MathValueRecord constant. */
export type ValueConstant =
  (typeof CONSTANT_GROUPS)[keyof typeof CONSTANT_GROUPS][number]

/** Constants stored as plain integers rather than value records. */
export const PLAIN_CONSTANTS = [
  'script_percent_scale_down',
  'script_script_percent_scale_down',
  'radical_degree_bottom_raise_percent',
  'delimited_sub_formula_min_height',
  'display_operator_min_height',
] as const satisfies readonly (keyof MathConstants)[]

const zero = (): MathValue => ({ value: 0, device: null })

/** A complete constants record with every value at zero. */
export function emptyConstants(): MathConstants {
  const constants: Record<string, number | MathValue> = {}
  for (const name of PLAIN_CONSTANTS) constants[name] = 0
  for (const group of Object.values(CONSTANT_GROUPS))
    for (const name of group) constants[name] = zero()
  return constants as unknown as MathConstants
}

/** A new table with zero constants and no glyph records. */
export function emptyTable(): MathTable {
  return { constants: emptyConstants(), min_connector_overlap: 0, glyphs: {} }
}

/** An empty glyph record. */
export function emptyGlyph(): MathGlyphInfo {
  return {
    italic_correction: null,
    top_accent: null,
    extended_shape: false,
    kern: {
      top_right: null,
      top_left: null,
      bottom_right: null,
      bottom_left: null,
    },
    vertical: null,
    horizontal: null,
  }
}

/** Device text is `start: delta delta ...`, or empty for no corrections. */
export function formatDevice(device: MathDevice | null | undefined): string {
  return device ? `${device.start_size}: ${device.deltas.join(' ')}` : ''
}

/**
 * Parse device text; `undefined` means the text is invalid.
 *
 * Corrections are signed 8-bit pixel counts for consecutive sizes that must
 * end at or before PPEM 65535, matching the model validation.
 */
export function parseDevice(text: string): MathDevice | null | undefined {
  if (text.trim() === '') return null
  const match = /^\s*(\d+)\s*:\s*(-?\d+(?:[\s,]+-?\d+)*)\s*$/.exec(text)
  if (!match) return undefined
  const start = Number(match[1])
  const deltas = match[2]!.split(/[\s,]+/).map(Number)
  if (
    start > 65535 ||
    start + deltas.length > 65536 ||
    deltas.some((delta) => delta < -128 || delta > 127)
  )
    return undefined
  return { start_size: start, deltas }
}

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b)

/**
 * Commands that turn the document's table into `draft` with the fewest
 * whole records.
 *
 * Creating or deleting the table replaces it; deleting also covers an
 * imported raw table, which exists although `state.math` is null. Otherwise
 * changed constants and changed glyph records are sent individually.
 */
export function diffCommands(
  state: Pick<MathState, 'math' | 'status'>,
  draft: MathTable | null,
): MathCommand[] {
  const source = state.math
  if (source === null || draft === null) {
    if (draft === null && source === null && state.status !== 'Imported')
      return []
    return [{ Replace: { math: draft } }]
  }
  const commands: MathCommand[] = []
  if (
    !same(source.constants, draft.constants) ||
    source.min_connector_overlap !== draft.min_connector_overlap
  )
    commands.push({
      SetGlobals: {
        constants: draft.constants,
        min_connector_overlap: draft.min_connector_overlap,
      },
    })
  const names = new Set([
    ...Object.keys(source.glyphs),
    ...Object.keys(draft.glyphs),
  ])
  for (const glyph of [...names].sort()) {
    const before = source.glyphs[glyph] ?? null
    const after = draft.glyphs[glyph] ?? null
    if (!same(before, after))
      commands.push({ SetGlyph: { glyph, info: after } })
  }
  return commands
}
