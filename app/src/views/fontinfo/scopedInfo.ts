import type { Axis, FontInfo, Instance, Master } from '../../api'

import { userToDesign } from './designCoordinates'
import {
  resolveMasterMetric,
  type OptionalMasterMetric,
} from './metricInheritance'

/**
 * The four scopes a value can come from.
 *
 * `family` holds what every export of the family shares, `font` this source's
 * own identity, `master` the per-master drawing metrics, and `export` the
 * per-instance values the compiler applies last.
 */
export type InfoScope = 'family' | 'font' | 'master' | 'export'

/** One source's contribution to a field. */
export interface ScopeEntry {
  /** Master or instance name. */
  name: string
  /** Value as display text. */
  value: string
  /** Whether this value actually differs from the one it would inherit. */
  overridden: boolean
  /** Whether the compiler derives it rather than reading a stored value. */
  derived?: boolean
}

/** One info field and the scopes that define it, in inheritance order. */
export interface ScopedField {
  /** Translation key suffix under `fontInfo.scoped.field`. */
  id: string
  /** Family-wide value, when this field has one. */
  family?: string
  /** This source's own value, or the default every master inherits. */
  font?: string
  /** Per-master values, when this field is a master metric. */
  master?: ScopeEntry[]
  /** Per-instance values, when exports may change this field. */
  export?: ScopeEntry[]
  /** Scope that supplies what an export actually receives. */
  effective: InfoScope
}

const DASH = '—'

function text(value: string | null | undefined): string {
  const trimmed = value?.trim()
  return trimmed ? trimmed : DASH
}

function num(value: number | null | undefined): string {
  return value === null || value === undefined ? DASH : String(value)
}

/** Axis identity as the design-space API reports it. */
function axisTag(tag: Axis['tag']): string {
  return typeof tag === 'string' ? tag : String.fromCharCode(...tag)
}

/** Fields OpenType defines for the whole family. */
const FAMILY_FIELDS: [string, (info: FontInfo) => string][] = [
  ['familyName', (info) => text(info.family_name)],
  [
    'preferredFamily',
    (info) => text(info.open_type_name_preferred_family_name),
  ],
  ['wwsFamily', (info) => text(info.open_type_name_wws_family_name)],
  ['designer', (info) => text(info.open_type_name_designer)],
  ['designerUrl', (info) => text(info.open_type_name_designer_url)],
  ['manufacturer', (info) => text(info.open_type_name_manufacturer)],
  ['manufacturerUrl', (info) => text(info.open_type_name_manufacturer_url)],
  ['license', (info) => text(info.open_type_name_license)],
  ['licenseUrl', (info) => text(info.open_type_name_license_url)],
  ['copyright', (info) => text(info.copyright)],
  ['trademark', (info) => text(info.trademark)],
  ['vendorId', (info) => text(info.open_type_os2_vendor_id)],
  ['version', (info) => `${info.version_major}.${info.version_minor}`],
]

/** Fields that belong to this source rather than to the family. */
const FONT_FIELDS: [string, (info: FontInfo) => string][] = [
  ['styleName', (info) => text(info.style_name)],
  [
    'preferredSubfamily',
    (info) => text(info.open_type_name_preferred_subfamily_name),
  ],
  ['wwsSubfamily', (info) => text(info.open_type_name_wws_subfamily_name)],
  ['postscriptFontName', (info) => text(info.postscript_font_name)],
  ['postscriptFullName', (info) => text(info.postscript_full_name)],
  ['uniqueId', (info) => text(info.open_type_name_unique_id)],
  ['unitsPerEm', (info) => num(info.units_per_em)],
  ['weightClass', (info) => num(info.open_type_os2_weight_class)],
  ['widthClass', (info) => num(info.open_type_os2_width_class)],
]

/** Required master metrics, which every master states outright. */
const REQUIRED_METRICS: [string, (master: Master) => number][] = [
  ['ascender', (master) => master.metrics.ascender],
  ['descender', (master) => master.metrics.descender],
  ['xHeight', (master) => master.metrics.x_height],
  ['capHeight', (master) => master.metrics.cap_height],
  ['italicAngle', (master) => master.metrics.italic_angle],
]

/** Optional master metrics, which inherit a computed default until overridden. */
const OPTIONAL_METRICS: [string, OptionalMasterMetric][] = [
  ['hheaAscender', 'hhea_ascender'],
  ['hheaDescender', 'hhea_descender'],
  ['hheaLineGap', 'hhea_line_gap'],
  ['typoAscender', 'typo_ascender'],
  ['typoDescender', 'typo_descender'],
  ['typoLineGap', 'typo_line_gap'],
  ['winAscent', 'win_ascent'],
  ['winDescent', 'win_descent'],
  ['verticalOrigin', 'default_vertical_origin'],
  ['verticalAdvance', 'default_vertical_advance'],
]

/** The `usWidthClass` percentages OpenType defines, in class order. */
const WIDTH_CLASS_PERCENTAGES = [50, 62.5, 75, 87.5, 100, 112.5, 125, 150, 200]

/** Map a user-space `wdth` percentage onto its OpenType width class, 1 to 9. */
export function widthClassOf(percentage: number): number {
  const value = Math.min(Math.max(percentage, 50), 200)
  for (let index = 1; index < WIDTH_CLASS_PERCENTAGES.length; index += 1) {
    const low = WIDTH_CLASS_PERCENTAGES[index - 1]!
    const high = WIDTH_CLASS_PERCENTAGES[index]!
    if (value <= high) return Math.round(index + (value - low) / (high - low))
  }
  return WIDTH_CLASS_PERCENTAGES.length
}

/** Invert an axis mapping: instance locations are design coordinates. */
function designToUser(axis: Axis, value: number): number {
  const map = axis.map
  if (!map.length) return value
  const first = map[0]!
  const last = map[map.length - 1]!
  if (value <= first[1]) return value + first[0] - first[1]
  for (let index = 1; index < map.length; index += 1) {
    const before = map[index - 1]!
    const after = map[index]!
    if (value <= after[1])
      return (
        before[0] +
        ((value - before[1]) / (after[1] - before[1])) * (after[0] - before[0])
      )
  }
  return value + last[0] - last[1]
}

/** One instance's user-space position on a registered axis, when it has one. */
function userCoordinate(
  axes: readonly Axis[],
  instance: Instance,
  tag: string,
): number | null {
  const axis = axes.find((candidate) => axisTag(candidate.tag) === tag)
  if (!axis) return null
  const design = instance.location[tag] ?? userToDesign(axis, axis.default)
  return designToUser(axis, design)
}

/**
 * The master an export inherits its metrics from.
 *
 * The design default is the master at every axis' default coordinate; a static
 * font has exactly one master to inherit from.
 */
export function defaultMaster(
  masters: readonly Master[],
  axes: readonly Axis[],
): Master | undefined {
  if (axes.length === 0) return masters[0]
  return (
    masters.find((master) =>
      axes.every((axis) => {
        const tag = axisTag(axis.tag)
        const value = master.location[tag] ?? userToDesign(axis, axis.default)
        return value === userToDesign(axis, axis.default)
      }),
    ) ?? masters[0]
  )
}

function exportValue(value: string | number | null | undefined): string | null {
  if (value === null || value === undefined) return null
  const text = String(value).trim()
  return text ? text : null
}

/**
 * Every info field with the scope that supplies its effective value.
 *
 * A field reports `export` as effective only when some instance's value really
 * differs from the one it would inherit, so an instance that merely repeats the
 * font's own family name is not reported as an override.
 */
export function scopedFields(
  info: FontInfo,
  masters: readonly Master[],
  instances: readonly Instance[],
  axes: readonly Axis[] = [],
): ScopedField[] {
  const fields: ScopedField[] = []
  /** Explicit export values, paired with what they would otherwise inherit. */
  const explicit: Record<
    string,
    { pick: (instance: Instance) => string | null; inherited: string }
  > = {
    familyName: {
      pick: (instance) => exportValue(instance.family_name),
      inherited: text(info.family_name),
    },
    preferredFamily: {
      pick: (instance) =>
        exportValue(instance.overrides?.preferred_family_name),
      inherited: text(info.open_type_name_preferred_family_name),
    },
    styleName: {
      pick: (instance) => exportValue(instance.style_name),
      inherited: text(info.style_name),
    },
    preferredSubfamily: {
      pick: (instance) =>
        exportValue(instance.overrides?.preferred_subfamily_name),
      inherited: text(info.open_type_name_preferred_subfamily_name),
    },
    postscriptFontName: {
      pick: (instance) => exportValue(instance.overrides?.postscript_font_name),
      inherited: text(info.postscript_font_name),
    },
  }
  /**
   * Classes the compiler derives from the instance location, which an explicit
   * override then replaces — the order `fc-compile` applies them in.
   */
  const derivedClasses: Record<
    string,
    { of: (instance: Instance) => number | null; inherited: string }
  > = {
    weightClass: {
      of: (instance) => {
        const weight = userCoordinate(axes, instance, 'wght')
        return weight === null
          ? null
          : Math.min(Math.max(Math.round(weight), 1), 1000)
      },
      inherited: num(info.open_type_os2_weight_class),
    },
    widthClass: {
      of: (instance) => {
        const width = userCoordinate(axes, instance, 'wdth')
        return width === null ? null : widthClassOf(width)
      },
      inherited: num(info.open_type_os2_width_class),
    },
  }

  const entriesFor = (id: string): ScopeEntry[] => {
    const plain = explicit[id]
    if (plain)
      return instances.flatMap((instance) => {
        const value = plain.pick(instance)
        if (value === null || value === plain.inherited) return []
        return [{ name: instance.name, value, overridden: true }]
      })
    const derived = derivedClasses[id]
    if (!derived) return []
    return instances.flatMap((instance) => {
      const override = exportValue(
        id === 'weightClass'
          ? instance.overrides?.weight_class
          : instance.overrides?.width_class,
      )
      const computed = derived.of(instance)
      const value = override ?? (computed === null ? null : String(computed))
      if (value === null) return []
      return [
        {
          name: instance.name,
          value,
          overridden: value !== derived.inherited,
          derived: override === null,
        },
      ]
    })
  }

  for (const [id, read] of FAMILY_FIELDS) {
    const entries = entriesFor(id)
    fields.push({
      id,
      family: read(info),
      ...(entries.length ? { export: entries } : {}),
      effective: entries.some((entry) => entry.overridden)
        ? 'export'
        : 'family',
    })
  }
  for (const [id, read] of FONT_FIELDS) {
    const entries = entriesFor(id)
    fields.push({
      id,
      font: read(info),
      ...(entries.length ? { export: entries } : {}),
      effective: entries.some((entry) => entry.overridden) ? 'export' : 'font',
    })
  }
  const inherited = defaultMaster(masters, axes)
  for (const [id, read] of REQUIRED_METRICS) {
    const base = inherited ? read(inherited) : null
    fields.push({
      id,
      ...(base === null ? {} : { font: String(base) }),
      master: masters.map((master) => ({
        name: master.name,
        value: String(read(master)),
        overridden: base !== null && read(master) !== base,
      })),
      effective: 'master',
    })
  }
  for (const [id, field] of OPTIONAL_METRICS) {
    const entries = masters.map((master) => {
      const resolved = resolveMasterMetric(
        master.metrics,
        field,
        info.units_per_em,
      )
      return {
        name: master.name,
        value: String(resolved.value),
        overridden: resolved.overridden,
      }
    })
    // The automatic value each master would fall back to; shown at the font
    // scope only when every master computes the same one.
    const defaults = masters.map(
      (master) =>
        resolveMasterMetric(
          { ...master.metrics, [field]: null },
          field,
          info.units_per_em,
        ).value,
    )
    const shared =
      defaults.length > 0 && defaults.every((value) => value === defaults[0])
        ? String(defaults[0])
        : null
    fields.push({
      id,
      ...(shared === null ? {} : { font: shared }),
      master: entries,
      effective: 'master',
    })
  }
  return fields
}

/** Filter the field list by a case-insensitive query over label and values. */
export function matchingFields(
  fields: readonly ScopedField[],
  labels: (id: string) => string,
  query: string,
): ScopedField[] {
  const needle = query.trim().toLowerCase()
  if (!needle) return [...fields]
  return fields.filter((field) => {
    const haystack = [
      labels(field.id),
      field.family ?? '',
      field.font ?? '',
      ...(field.master ?? []).flatMap((entry) => [entry.name, entry.value]),
      ...(field.export ?? []).flatMap((entry) => [entry.name, entry.value]),
    ]
      .join(' ')
      .toLowerCase()
    return haystack.includes(needle)
  })
}

/**
 * Whether an instance's family name really replaces the font's.
 *
 * A designspace import writes the font's own family into every instance, so
 * presence alone is not an override: only a different name is one.
 */
export function overridesFamily(
  instanceFamily: string | null,
  fontFamily: string,
): boolean {
  return instanceFamily !== null && instanceFamily.trim() !== fontFamily.trim()
}

/** Whether a style name says more than the instance name already does. */
export function overridesStyle(
  styleName: string,
  instanceName: string,
): boolean {
  const style = styleName.trim()
  return style !== '' && style !== instanceName.trim()
}
