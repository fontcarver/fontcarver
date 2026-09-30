import { describe, expect, it } from 'vitest'

import type { Axis, Instance } from '../../api'
import { makeFontInfo, makeInstance, makeMaster } from '../../test/fontFixtures'

import {
  defaultMaster,
  matchingFields,
  overridesFamily,
  overridesStyle,
  scopedFields,
  widthClassOf,
} from './scopedInfo'

function instance(overrides: Partial<Instance>): Instance {
  return makeInstance(overrides)
}

const info = makeFontInfo({
  family_name: 'Scope Family',
  style_name: 'Regular',
  units_per_em: 1000,
  open_type_os2_weight_class: 400,
  open_type_os2_width_class: 5,
  open_type_name_designer: 'A Designer',
})
const masters = [
  makeMaster({ id: 'm1', name: 'Regular' }),
  makeMaster({ id: 'm2', name: 'Bold' }),
]

/** A weight axis with no mapping, so design and user coordinates agree. */
const weightAxis: Axis = {
  tag: [119, 103, 104, 116] as [number, number, number, number],
  name: 'Weight',
  min: 400,
  default: 400,
  max: 700,
  map: [],
  hidden: false,
}
const widthAxis: Axis = {
  ...weightAxis,
  tag: [119, 100, 116, 104],
  name: 'Width',
  min: 75,
  default: 100,
  max: 100,
}

function field(
  id: string,
  instances: readonly Instance[] = [],
  axes: readonly Axis[] = [],
) {
  const found = scopedFields(info, masters, instances, axes).find(
    (candidate) => candidate.id === id,
  )
  if (!found) throw new Error(`no scoped field ${id}`)
  return found
}

describe('scoped info', () => {
  it('reports a family-wide field at the family scope', () => {
    expect(field('familyName')).toMatchObject({
      family: 'Scope Family',
      effective: 'family',
    })
    expect(field('designer')).toMatchObject({
      family: 'A Designer',
      effective: 'family',
    })
    expect(field('familyName').font).toBeUndefined()
  })

  it("reports this source's own identity at the font scope", () => {
    expect(field('styleName')).toMatchObject({
      font: 'Regular',
      effective: 'font',
    })
    expect(field('unitsPerEm')).toMatchObject({
      font: '1000',
      effective: 'font',
    })
  })

  it('does not call a repeated value an override', () => {
    // Every instance carries the font's own family name, as a designspace
    // import writes it. Nothing differs, so nothing is an override.
    const repeated = [
      instance({ name: 'Bold', family_name: 'Scope Family' }),
      instance({ name: 'Light', family_name: 'Scope Family' }),
    ]
    const family = field('familyName', repeated)
    expect(family.effective).toBe('family')
    expect(family.export).toBeUndefined()
    // The same holds for a style name an instance merely repeats.
    expect(
      field('styleName', [instance({ name: 'Regular', style_name: 'Regular' })])
        .effective,
    ).toBe('font')
  })

  it('reports a differing instance value as the effective export scope', () => {
    const instances = [
      instance({ name: 'Bold', family_name: 'Scope Display' }),
      instance({ name: 'Light', family_name: 'Scope Family' }),
    ]
    const family = field('familyName', instances)
    expect(family.effective).toBe('export')
    expect(family.family).toBe('Scope Family')
    // Only the instance that actually differs appears in the export cell.
    expect(family.export).toEqual([
      { name: 'Bold', value: 'Scope Display', overridden: true },
    ])
  })

  it('shows the weight and width classes the compiler derives from the location', () => {
    const instances = [
      instance({ name: 'Regular', location: { wght: 400, wdth: 100 } }),
      instance({ name: 'Bold', location: { wght: 700, wdth: 100 } }),
      instance({ name: 'Condensed', location: { wght: 400, wdth: 75 } }),
    ]
    const axes = [weightAxis, widthAxis]
    const weight = field('weightClass', instances, axes)
    expect(weight.font).toBe('400')
    expect(weight.export).toEqual([
      { name: 'Regular', value: '400', overridden: false, derived: true },
      { name: 'Bold', value: '700', overridden: true, derived: true },
      { name: 'Condensed', value: '400', overridden: false, derived: true },
    ])
    expect(weight.effective).toBe('export')
    const width = field('widthClass', instances, axes)
    expect(width.export?.map((entry) => entry.value)).toEqual(['5', '5', '3'])
    expect(width.export?.map((entry) => entry.overridden)).toEqual([
      false,
      false,
      true,
    ])
  })

  it('lets an explicit class override replace the derived one', () => {
    const instances = [
      instance({
        name: 'Black',
        location: { wght: 700 },
        overrides: { weight_class: 900 },
      }),
    ]
    expect(field('weightClass', instances, [weightAxis]).export).toEqual([
      { name: 'Black', value: '900', overridden: true, derived: false },
    ])
  })

  it('reports no class entries for a font without those axes', () => {
    const instances = [instance({ name: 'Plain' })]
    expect(field('weightClass', instances)).toMatchObject({ effective: 'font' })
    expect(field('weightClass', instances).export).toBeUndefined()
  })

  it('maps a width percentage onto its OpenType class', () => {
    expect(widthClassOf(50)).toBe(1)
    expect(widthClassOf(75)).toBe(3)
    expect(widthClassOf(100)).toBe(5)
    expect(widthClassOf(150)).toBe(8)
    expect(widthClassOf(200)).toBe(9)
    // Values outside the registered range clamp rather than extrapolate.
    expect(widthClassOf(10)).toBe(1)
    expect(widthClassOf(400)).toBe(9)
  })

  it('reports a required metric against the default master, not always', () => {
    const differing = [
      makeMaster({ id: 'm1', name: 'Regular' }),
      makeMaster({
        id: 'm2',
        name: 'Bold',
        metrics: { ...makeMaster().metrics, cap_height: 720 },
      }),
    ]
    const found = scopedFields(info, differing, [], []).find(
      (candidate) => candidate.id === 'capHeight',
    )
    expect(found?.font).toBe(String(makeMaster().metrics.cap_height))
    expect(found?.master?.[0]).toMatchObject({
      name: 'Regular',
      overridden: false,
    })
    expect(found?.master?.[1]).toMatchObject({
      name: 'Bold',
      value: '720',
      overridden: true,
    })
    // Masters that agree report no override at all.
    const same = scopedFields(info, masters, [], []).find(
      (candidate) => candidate.id === 'ascender',
    )
    expect(same?.master?.every((entry) => !entry.overridden)).toBe(true)
  })

  it('shows the automatic default of an optional metric at the font scope', () => {
    const overridden = [
      makeMaster({ id: 'm1', name: 'Regular' }),
      makeMaster({
        id: 'm2',
        name: 'Bold',
        metrics: { ...makeMaster().metrics, typo_ascender: 777 },
      }),
    ]
    const found = scopedFields(info, overridden, [], []).find(
      (candidate) => candidate.id === 'typoAscender',
    )
    expect(found?.effective).toBe('master')
    // Both masters compute the same automatic value, so it is shown once.
    expect(found?.font).toBe(String(makeMaster().metrics.ascender))
    expect(found?.master?.[1]).toMatchObject({
      name: 'Bold',
      value: '777',
      overridden: true,
    })
    // The vertical advance falls back to the font's units per em.
    const advance = scopedFields(info, masters, [], []).find(
      (candidate) => candidate.id === 'verticalAdvance',
    )
    expect(advance?.font).toBe('1000')
  })

  it('picks the master at every axis default as the one exports inherit', () => {
    const light = makeMaster({
      id: 'm0',
      name: 'Light',
      location: { wght: 100 },
    })
    const regular = makeMaster({
      id: 'm1',
      name: 'Regular',
      location: { wght: 400 },
    })
    expect(defaultMaster([light, regular], [weightAxis])?.name).toBe('Regular')
    // A static font inherits from its only master.
    expect(defaultMaster([light], [])?.name).toBe('Light')
  })

  it('filters by label, value, master name and instance name', () => {
    const instances = [instance({ name: 'Bold', family_name: 'Display' })]
    const fields = scopedFields(info, masters, instances, [])
    const labels = (id: string) => id
    expect(
      matchingFields(fields, labels, 'familyName').map((field) => field.id),
    ).toEqual(['familyName'])
    expect(
      matchingFields(fields, labels, 'Display').map((field) => field.id),
    ).toEqual(['familyName'])
    expect(
      matchingFields(fields, labels, 'A Designer').map((field) => field.id),
    ).toEqual(['designer'])
    expect(matchingFields(fields, labels, '   ')).toHaveLength(fields.length)
    expect(matchingFields(fields, labels, 'nothing here')).toEqual([])
  })
})

describe('instance override labels', () => {
  it('calls a repeated family or style name inherited, not an override', () => {
    // A designspace import writes both into every instance.
    expect(overridesFamily('Scope Family', 'Scope Family')).toBe(false)
    expect(overridesFamily(null, 'Scope Family')).toBe(false)
    expect(overridesFamily('Scope Display', 'Scope Family')).toBe(true)
    expect(overridesStyle('Bold', 'Bold')).toBe(false)
    expect(overridesStyle('', 'Bold')).toBe(false)
    expect(overridesStyle('Heavy', 'Bold')).toBe(true)
  })
})
