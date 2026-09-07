import type { DocId, GlyphCell, GlyphId, MasterId, ShapedRun } from '../../api'
import type { Transport } from '../../transport'

import { parseTextInput } from './textInput'

export interface TextRequestContext {
  doc: DocId
  /** Current document generation, used to scope async diagnostics. */
  generation: number
  /** Reliable document UPEM, including when the input contains only names. */
  unitsPerEm: number
  master: MasterId | null
  text: string
  features: Array<[string, boolean]>
  direction: string | null
  script: string | null
  language: string | null
}

export type TextDiagnosticCode =
  | 'shape-warning'
  | 'compile-failed'
  | 'unknown-glyph-name'
  | 'unresolved-shaped-name'
  | 'missing-compiled-name'

export interface TextDiagnostic {
  code: TextDiagnosticCode
  message: string
  name?: string
  sourceByte?: number
}

export interface PositionedGlyph {
  /** Compiled-font index, retained for diagnostics and never used for editing. */
  compiledGid: number | null
  /** Stable session handle, present only after an exact-name API match. */
  documentGid: GlyphId | null
  name: string | null
  editable: boolean
  x: number
  y: number
  xAdvance: number
  yAdvance: number
  sourceByte: number
  sourceKind: 'literal' | 'named'
}

export interface PositionedRun {
  glyphs: PositionedGlyph[]
  end: { x: number; y: number }
}

export interface TextLayout extends PositionedRun {
  diagnostics: TextDiagnostic[]
  stale: boolean
  unitsPerEm: number
}

export type LayoutUnits = 'font' | 'run'

/** Scale from a shaped run's returned units to document font units. */
export function runUnitsToFontUnits(run: ShapedRun): number {
  return run.size === null || run.size <= 0 ? 1 : run.units_per_em / run.size
}

/** Positions one shaped run in visual order using advances and offsets. */
export function positionShapedRun(
  run: ShapedRun,
  origin = { x: 0, y: 0 },
  sourceBytes: number | readonly number[] = 0,
  units: LayoutUnits = 'run',
): PositionedRun {
  const scale = units === 'font' ? runUnitsToFontUnits(run) : 1
  let penX = origin.x
  let penY = origin.y
  const glyphs = run.glyphs.map((glyph, index): PositionedGlyph => {
    const positioned = {
      compiledGid: glyph.gid,
      documentGid: null,
      name: run.names[index] ?? null,
      editable: false,
      x: penX + glyph.x_offset * scale,
      y: penY + glyph.y_offset * scale,
      xAdvance: glyph.x_advance * scale,
      yAdvance: glyph.y_advance * scale,
      sourceByte:
        typeof sourceBytes === 'number'
          ? sourceBytes + glyph.cluster
          : (sourceBytes[glyph.cluster] ?? sourceBytes.at(-1) ?? 0),
      sourceKind: 'literal' as const,
    }
    penX += glyph.x_advance * scale
    penY += glyph.y_advance * scale
    return positioned
  })
  return { glyphs, end: { x: penX, y: penY } }
}

async function exactGlyphCell(
  transport: Transport,
  context: TextRequestContext,
  name: string,
): Promise<GlyphCell | null> {
  const matches = await transport.call('glyphs.search', {
    doc: context.doc,
    query: name,
    limit: 32,
  })
  const exact = matches.find((cell) => cell.name === name)
  if (!exact) return null
  if (context.master === null) return exact
  const cell = await transport.call('glyphs.cell', {
    doc: context.doc,
    gid: exact.gid,
    master: context.master,
  })
  return cell.name === name ? cell : null
}

/**
 * Shapes all literal spans and resolves every editable glyph through an exact
 * document-name lookup. `ShapedGlyph.gid` is intentionally never promoted to a
 * document handle because it belongs to the transient compiled font.
 */
export async function buildTextLayout(
  transport: Transport,
  context: TextRequestContext,
): Promise<TextLayout> {
  const diagnostics: TextDiagnostic[] = []
  const glyphs: PositionedGlyph[] = []
  let pen = { x: 0, y: 0 }
  let stale = false
  let unitsPerEm = context.unitsPerEm
  const tokens = parseTextInput(context.text)
  const shapedRuns = await Promise.all(
    tokens.map((token) =>
      token.kind === 'named'
        ? null
        : transport.call('text.shape', {
            doc: context.doc,
            text: token.text,
            master: context.master,
            features: context.features,
            script: context.script,
            language: context.language,
            direction: context.direction,
            size: null,
          }),
    ),
  )
  const names = new Set<string>()
  tokens.forEach((token, index) => {
    if (token.kind === 'named') names.add(token.name)
    else
      for (const name of shapedRuns[index]?.names ?? [])
        if (name !== null) names.add(name)
  })
  const resolvedEntries = await Promise.all(
    [...names].map(
      async (name) =>
        [name, await exactGlyphCell(transport, context, name)] as const,
    ),
  )
  const resolutions = new Map(resolvedEntries)

  for (const [tokenIndex, token] of tokens.entries()) {
    if (token.kind === 'named') {
      const cell = resolutions.get(token.name) ?? null
      if (!cell) {
        diagnostics.push({
          code: 'unknown-glyph-name',
          message: `Glyph “${token.name}” does not exist in this document.`,
          name: token.name,
          sourceByte: token.sourceByteStart,
        })
      }
      const advance = cell?.width ?? 0
      glyphs.push({
        compiledGid: null,
        documentGid: cell?.gid ?? null,
        name: token.name,
        editable: cell !== null,
        x: pen.x,
        y: pen.y,
        xAdvance: advance,
        yAdvance: 0,
        sourceByte: token.sourceByteStart,
        sourceKind: 'named',
      })
      pen = { x: pen.x + advance, y: pen.y }
      continue
    }

    const shaped = shapedRuns[tokenIndex]
    if (!shaped) continue
    stale ||= shaped.stale
    unitsPerEm = shaped.units_per_em
    for (const warning of shaped.warnings) {
      diagnostics.push({
        code: warning.startsWith('compile-failed:')
          ? 'compile-failed'
          : 'shape-warning',
        message: warning,
      })
    }

    const positioned = positionShapedRun(
      shaped,
      pen,
      token.sourceByteMap,
      'font',
    )
    for (const glyph of positioned.glyphs) {
      if (glyph.name === null) {
        diagnostics.push({
          code: 'missing-compiled-name',
          message:
            'The compiled font did not provide an exact glyph name; this glyph cannot be edited.',
          sourceByte: glyph.sourceByte,
        })
      } else {
        const cell = resolutions.get(glyph.name) ?? null
        if (cell) {
          glyph.documentGid = cell.gid
          glyph.editable = true
        } else {
          diagnostics.push({
            code: 'unresolved-shaped-name',
            message: `Shaped glyph “${glyph.name}” has no exact document match and cannot be edited.`,
            name: glyph.name,
            sourceByte: glyph.sourceByte,
          })
        }
      }
      glyphs.push(glyph)
    }
    pen = positioned.end
  }

  return { glyphs, end: pen, diagnostics, stale, unitsPerEm }
}
