import type { ExportFormat, ExportOptions } from '../api'
import type { ExportOptionField } from './ExportProcessingOptions'
import {
  ALL_MASTER_FORMATS,
  SFNT_FORMATS,
  TYPE1_FORMATS,
} from './exportFormats'

/** User choices retained when a format temporarily disables an option. */
export const DEFAULT_EXPORT_PROCESSING: Record<ExportOptionField, boolean> = {
  remove_overlap: false,
  add_extrema: false,
  round_coordinates: false,
  decompose_components: false,
  subroutinize: false,
  autohint: false,
  keep_tt_instructions: true,
  preserve_untouched_tables: true,
  production_names: false,
}

/** Form values normalized at the API boundary, also used for saved profiles. */
export function exportOptionsFor(values: {
  format: ExportFormat
  canVary: boolean
  variable: boolean
  instance: number | undefined
  masterSubset: string[] | undefined
  bitmapKey: string
  master: string | null
  glyph: string
  options: Record<ExportOptionField, boolean>
}): ExportOptions {
  const {
    format,
    canVary,
    variable,
    instance,
    masterSubset,
    bitmapKey,
    master,
    glyph,
    options,
  } = values
  const sfnt = SFNT_FORMATS.includes(format)
  const preprocess =
    (sfnt || TYPE1_FORMATS.includes(format)) && !(canVary && variable)
  return {
    instance: canVary && !variable ? instance : undefined,
    master_subset: canVary && variable ? masterSubset?.slice() : undefined,
    variable: canVary && variable,
    format,
    glyph: format === 'Eps' && glyph.trim() ? glyph.trim() : undefined,
    bitmap_strike:
      ['Bdf', 'Pcf', 'Psf', 'Psf1', 'Fnt'].includes(format) && bitmapKey
        ? (bitmapKey.split(':').map(Number) as [number, number])
        : undefined,
    master: ALL_MASTER_FORMATS.includes(format) ? null : master,
    remove_overlap: preprocess && options.remove_overlap,
    add_extrema: preprocess && options.add_extrema,
    round_coordinates: preprocess && options.round_coordinates,
    decompose_components: preprocess && options.decompose_components,
    subroutinize:
      preprocess && ['Otf', 'Cff'].includes(format) && options.subroutinize,
    keep_tt_instructions:
      sfnt &&
      !(canVary && (variable || instance !== undefined)) &&
      !['Otf', 'Cff'].includes(format) &&
      options.keep_tt_instructions,
    preserve_untouched_tables:
      sfnt && format !== 'Cff' && options.preserve_untouched_tables,
    production_names: sfnt && options.production_names,
    autohint: preprocess && options.autohint,
    table_subset: null,
  }
}

/** Restores supported processing choices from a persisted profile. */
export function processingFromProfile(
  options: ExportOptions,
): Record<ExportOptionField, boolean> {
  return Object.fromEntries(
    Object.entries(DEFAULT_EXPORT_PROCESSING).map(([field, fallback]) => [
      field,
      options[field as ExportOptionField] ?? fallback,
    ]),
  ) as Record<ExportOptionField, boolean>
}
