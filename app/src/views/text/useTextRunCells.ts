import { useMemo } from 'react'
import type { DocId, MasterId } from '../../api'
import { useGlyphColors } from '../color/useGlyphColor'
import type { PositionedGlyph } from './textLayout'
import { MAX_VISIBLE_RUN_GLYPHS, previewGlyphIndexes } from './textRunGeometry'
import { useTextGlyphCells } from './useTextGlyphCells'

/** Shares bounded outline and color loading across the canvas and preview. */
export function useTextRunCells(
  doc: DocId,
  master: MasterId | null,
  generation: number,
  unitsPerEm: number,
  glyphs: readonly PositionedGlyph[],
  activeIndex: number | null,
  visible: readonly number[],
  previewVisible: boolean,
) {
  const previewIndexes = useMemo(
    () => previewGlyphIndexes(glyphs, unitsPerEm),
    [glyphs, unitsPerEm],
  )
  const wanted = useMemo(
    () =>
      [
        ...new Set([
          ...(activeIndex === null ? [] : [activeIndex]),
          ...visible,
          ...(previewVisible ? previewIndexes : []),
        ]),
      ].slice(0, MAX_VISIBLE_RUN_GLYPHS),
    [activeIndex, visible, previewVisible, previewIndexes],
  )
  const cells = useTextGlyphCells(doc, master, generation, glyphs, wanted)
  const colors = useGlyphColors(doc, master, generation, cells)
  const previewGlyphs = useMemo(
    () =>
      previewIndexes.flatMap((index) => {
        const glyph = glyphs[index]
        if (!glyph || glyph.documentGid === null) return []
        const path = cells.get(glyph.documentGid)?.path ?? ''
        const color = colors.get(glyph.documentGid)
        return path || color
          ? [{ ...glyph, path, ...(color ? { color } : {}) }]
          : []
      }),
    [cells, colors, glyphs, previewIndexes],
  )
  return { cells, colors, previewGlyphs }
}
