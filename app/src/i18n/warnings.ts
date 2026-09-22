import type { DocWarning, ExportWarning } from '../api'

/** Translation options supplied for a kernel warning. */
export interface WarningTranslationOptions {
  [key: string]: unknown
  defaultValue: string
  message: string
}

/** Minimal translation function used when rendering a kernel warning. */
export type WarningTranslator = (
  key: string,
  options?: WarningTranslationOptions,
) => string

const WARNING_KEYS: Readonly<Record<string, string>> = {
  // Import warnings.
  'cff-hints-not-imported': 'warning.cffHintsNotImported',
  'cff-private-dicts-flattened': 'warning.cffPrivateDictsFlattened',
  'cff-unhinted-outlines': 'warning.cffUnhintedOutlines',
  'tt-instruction-bindings-partial': 'warning.ttInstructionBindingsPartial',
  'tt-imported-programs-omitted': 'warning.ttImportedProgramsOmitted',
  'tt-authored-programs-omitted': 'warning.ttAuthoredProgramsOmitted',
  'type1-layout-loss': 'warning.type1LayoutLoss',
  'type1-hinting-loss': 'warning.type1HintingLoss',
  'type1-import-hints-partial': 'warning.type1ImportHintsPartial',
  'type1-import-private-transform': 'warning.type1ImportPrivateTransform',
  'type1-import-private-computed': 'warning.type1ImportPrivateComputed',
  'type1-unicode-loss': 'warning.type1UnicodeLoss',
  'glyph-names-synthesized': 'warning.glyphNamesSynthesized',
  'collection-face-opened': 'warning.collectionFaceOpened',
  // Export warnings. `ExportWarning` has the same shape as `DocWarning`, so the
  // export dialog renders its diagnostics through the same mapping.
  'hinting-dropped': 'warning.hintingDropped',
  'hinting-dropped-glyph-set-changed': 'warning.hintingDroppedGlyphSetChanged',
  'preservation-unavailable-no-provenance': 'warning.preservationUnavailable',
  'layout-dropped-glyph-order-changed': 'warning.layoutDropped',
  'raw-table-dropped-glyph-order-changed': 'warning.rawTableDropped',
  'variation-tables-dropped': 'warning.variationTablesDropped',
  'tables-not-preserved': 'warning.tablesNotPreserved',
  'provenance-hash-mismatch': 'warning.provenanceHashMismatch',
  'remove-overlap-unavailable': 'warning.removeOverlapUnavailable',
  'autohint-unavailable': 'warning.autohintUnavailable',
  'production-names-unavailable': 'warning.productionNamesUnavailable',
  'variation-sequences-dropped': 'warning.variationSequencesDropped',
  'post-names-omitted-capacity': 'warning.postNamesOmitted',
  'value-clamped': 'warning.valueClamped',
  'mac-name-records-dropped': 'warning.macNameRecordsDropped',
  'name-language-unmapped': 'warning.nameLanguageUnmapped',
  'invalid-horizontal-kerning': 'warning.invalidHorizontalKerning',
}

/** Translates a known warning code and preserves the kernel detail as fallback. */
export function warningMessage(
  warning: DocWarning | ExportWarning,
  t: WarningTranslator,
): string {
  const key = WARNING_KEYS[warning.code]
  return key
    ? t(key, { defaultValue: warning.message, message: warning.message })
    : warning.message
}
