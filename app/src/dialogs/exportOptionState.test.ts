import { expect, it } from 'vitest'
import {
  DEFAULT_EXPORT_PROCESSING,
  exportOptionsFor,
} from './exportOptionState'

const values = {
  format: 'Ttf' as const,
  canVary: true,
  variable: false,
  instance: undefined,
  masterSubset: undefined,
  axisSubset: undefined,
  instanceSubset: undefined,
  bitmapKey: '16:1',
  master: 'm1',
  glyph: ' A ',
  options: {
    ...DEFAULT_EXPORT_PROCESSING,
    add_extrema: true,
    round_coordinates: true,
    decompose_components: true,
    subroutinize: true,
    production_names: true,
  },
}

it('preserves supported static geometry processing and strips it from variable exports', () => {
  expect(exportOptionsFor(values)).toMatchObject({
    add_extrema: true,
    round_coordinates: true,
    decompose_components: true,
    subroutinize: false,
  })
  expect(exportOptionsFor({ ...values, variable: true })).toMatchObject({
    add_extrema: false,
    round_coordinates: false,
    decompose_components: false,
    subroutinize: false,
    keep_tt_instructions: false,
  })
})

it.each(['Otf', 'Cff'] as const)(
  'enables CFF subroutines for static %s only',
  (format) => {
    expect(exportOptionsFor({ ...values, format })).toMatchObject({
      subroutinize: true,
      keep_tt_instructions: false,
      preserve_untouched_tables: format !== 'Cff',
    })
    expect(
      exportOptionsFor({ ...values, format, variable: true }),
    ).toMatchObject({ subroutinize: false })
  },
)

it.each([
  'Ufo',
  'Svg',
  'Eps',
  'Afm',
  'Pfm',
  'Fcf',
  'Bdf',
  'Fon',
  'Cbdt',
] as const)(
  'does not send unsupported compiled processing for %s',
  (format) => {
    expect(
      exportOptionsFor({ ...values, canVary: false, format }),
    ).toMatchObject({
      add_extrema: false,
      round_coordinates: false,
      decompose_components: false,
      subroutinize: false,
      production_names: false,
      preserve_untouched_tables: false,
      keep_tt_instructions: false,
    })
  },
)

it('uses one bitmap strike only for single-strike formats and glyph names only for EPS', () => {
  expect(
    exportOptionsFor({ ...values, canVary: false, format: 'Bdf' })
      .bitmap_strike,
  ).toEqual([16, 1])
  expect(
    exportOptionsFor({ ...values, canVary: false, format: 'Fon' })
      .bitmap_strike,
  ).toBeUndefined()
  expect(
    exportOptionsFor({ ...values, canVary: false, format: 'Eps' }).glyph,
  ).toBe('A')
  expect(exportOptionsFor(values).glyph).toBeUndefined()
})

it('sends design-space subsets only for a variable export', () => {
  const subsets = {
    ...values,
    masterSubset: ['m1', 'm2'],
    axisSubset: ['wght'],
    instanceSubset: ['Bold'],
  }
  expect(exportOptionsFor({ ...subsets, variable: true })).toMatchObject({
    master_subset: ['m1', 'm2'],
    axis_subset: ['wght'],
    instance_subset: ['Bold'],
  })
  expect(exportOptionsFor(subsets)).toMatchObject({
    master_subset: undefined,
    axis_subset: undefined,
    instance_subset: undefined,
  })
  expect(
    exportOptionsFor({ ...subsets, canVary: false, variable: true }),
  ).toMatchObject({
    master_subset: undefined,
    axis_subset: undefined,
    instance_subset: undefined,
  })
})
