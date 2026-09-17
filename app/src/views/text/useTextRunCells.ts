import { useMemo } from 'react'
import type { DocId, MasterId } from '../../api'
import type { PositionedGlyph } from './textLayout'
import { MAX_VISIBLE_RUN_GLYPHS, previewGlyphIndexes } from './textRunGeometry'
import { useTextGlyphCells } from './useTextGlyphCells'

/** Shares bounded outline loading between the main canvas and preview strip. */
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
  const previewGlyphs = useMemo(
    () =>
      previewIndexes.flatMap((index) => {
        const glyph = glyphs[index]
        const path =
          glyph?.documentGid === null || !glyph
            ? undefined
            : cells.get(glyph.documentGid)?.path
        return glyph && path ? [{ ...glyph, path }] : []
      }),
    [cells, glyphs, previewIndexes],
  )
  return { cells, previewGlyphs }
}
