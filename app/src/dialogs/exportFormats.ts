import type { DocSummary, ExportFormat, ExportWarning } from '../api'
import { choosePath } from '../shell/nativeDialogs'

/** File-name extension each container format is written with. */
export const FORMAT_EXTENSIONS: Record<ExportFormat, string> = {
  Bdf: '.bdf',
  Pcf: '.pcf',
  Psf: '.psf',
  Psf1: '.psf',
  Fnt: '.fnt',
  Fon: '.fon',
  Otb: '.otb',
  Cbdt: '.ttf',
  Sbix: '.ttf',
  Ttf: '.ttf',
  Otf: '.otf',
  Woff: '.woff',
  Woff2: '.woff2',
  Ufo: '.ufo',
  Glyphs: '.glyphs',
  Sfd: '.sfd',
  Pfa: '.pfa',
  Pfb: '.pfb',
  Designspace: '.designspace',
  Fcf: '.fcf',
}

/** Formats compiled through the sfnt pipeline. */
export const SFNT_FORMATS: readonly ExportFormat[] = [
  'Ttf',
  'Otf',
  'Woff',
  'Woff2',
]

/** Static Type 1 outputs support overlap removal and PostScript automatic hints. */
export const TYPE1_FORMATS: readonly ExportFormat[] = ['Pfa', 'Pfb']

/** Source formats that always retain the document's complete master set. */
export const ALL_MASTER_FORMATS: readonly ExportFormat[] = [
  'Glyphs',
  'Designspace',
  'Fcf',
]

/** Formats written as a package directory rather than a single file. */
export const DIRECTORY_FORMATS: readonly ExportFormat[] = ['Ufo', 'Designspace']

export const BITMAP_FORMATS: readonly ExportFormat[] = [
  'Bdf',
  'Pcf',
  'Psf',
  'Psf1',
  'Fnt',
  'Fon',
  'Otb',
  'Cbdt',
  'Sbix',
]

const BASE_FORMATS: readonly ExportFormat[] = [
  'Bdf',
  'Pcf',
  'Psf',
  'Psf1',
  'Fnt',
  'Fon',
  'Otb',
  'Cbdt',
  'Sbix',
  'Ttf',
  'Otf',
  'Woff',
  'Woff2',
  'Ufo',
  'Glyphs',
  'Sfd',
  'Pfa',
  'Pfb',
  'Fcf',
]

/**
 * Format tabs to offer. A designspace needs more than one master to describe,
 * so the tab only appears for a multiple-master document.
 */
export function formatsFor(masterCount: number): ExportFormat[] {
  return masterCount > 1 ? [...BASE_FORMATS, 'Designspace'] : [...BASE_FORMATS]
}

function withoutExtension(name: string): string {
  const dot = name.lastIndexOf('.')
  return dot > 0 ? name.slice(0, dot) : name
}

/** A safe default destination: the document's own location and stem. */
export function suggestPath(summary: DocSummary, format: ExportFormat): string {
  const extension = FORMAT_EXTENSIONS[format]
  if (summary.path) {
    const separator = summary.path.includes('\\') ? '\\' : '/'
    const cut = summary.path.lastIndexOf(separator)
    const directory = cut < 0 ? '' : summary.path.slice(0, cut + 1)
    const stem = withoutExtension(summary.path.slice(cut + 1))
    return `${directory}${stem}${extension}`
  }
  const stem = `${summary.family_name}-${summary.style_name}`.replace(
    /[^\w.-]+/g,
    '',
  )
  return `${stem || 'Untitled'}${extension}`
}

/**
 * Codes whose kernel message ends in a comma-separated list of glyph names.
 *
 * `ExportWarning` carries a code and a human message, not a structured glyph
 * list, so the affected glyphs are read back out of the message. Only codes
 * known to format their list this way are parsed; every other warning simply
 * has no chips.
 */
const GLYPH_LIST_CODES: ReadonlySet<string> = new Set(['hinting-dropped'])

const GLYPH_NAME = /^[A-Za-z_.][\w.]{0,62}$/

/** Glyph names named by one export warning, in the order the kernel listed them. */
export function warningGlyphNames(warning: ExportWarning): string[] {
  if (!GLYPH_LIST_CODES.has(warning.code)) return []
  const colon = warning.message.lastIndexOf(': ')
  if (colon < 0) return []
  const names = warning.message
    .slice(colon + 2)
    .split(',')
    .map((token) => token.trim())
  return names.every((name) => GLYPH_NAME.test(name)) ? names : []
}

/**
 * Asks the platform for a destination, through the same helper Open and Save As
 * use. Returns `null` when the user cancelled or the platform has no native
 * dialog; the dialog's own destination field is the fallback in that case.
 */
export async function chooseExportPath(
  suggested: string,
  filterName: string,
  format: ExportFormat,
): Promise<string | null> {
  const choice = await choosePath({
    kind: 'save',
    filterName,
    extensions: [FORMAT_EXTENSIONS[format].slice(1)],
    defaultPath: suggested,
  })
  return choice.kind === 'chosen' ? choice.path : null
}
