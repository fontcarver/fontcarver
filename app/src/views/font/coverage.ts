/**
 * Repertoire coverage for the sidebar's present/total badges.
 *
 * `inventory.coverage` answers, for every category, subcategory, script and
 * Unicode block the font touches, how many of that repertoire's characters the
 * font encodes and how many it has in all. The numerator is characters, not
 * glyphs, so `present + missing = total` and "generate the missing ones" is a
 * well-defined action; the glyph count the matching filter selects travels
 * alongside it and is shown in the row's tooltip.
 */

import { useEffect, useState } from 'react'

import { call, type CoverageRow, type DocId } from '../../api'

/** How long coverage waits after an edit before it is recomputed. */
export const COVERAGE_REFRESH_MS = 400

/** Coverage rows of one grouping, keyed by repertoire name. */
export type CoverageIndex = ReadonlyMap<string, CoverageRow>

/** Everything the sidebar needs to badge its rows. */
export interface Coverage {
  categories: CoverageIndex
  subCategories: CoverageIndex
  scripts: CoverageIndex
  blocks: CoverageIndex
  /** Number of glyphs in the document, used as a filter's denominator. */
  totalGlyphs: number
}

const EMPTY: Coverage = {
  categories: new Map(),
  subCategories: new Map(),
  scripts: new Map(),
  blocks: new Map(),
  totalGlyphs: 0,
}

function index(rows: readonly CoverageRow[]): CoverageIndex {
  return new Map(rows.map((row) => [row.value, row]))
}

/**
 * Loads coverage for one document and keeps it roughly fresh.
 *
 * Coverage is derived from every glyph, so an edit invalidates it. The previous
 * numbers stay on screen until the new ones land and a burst of edits collapses
 * into one request — the same rule the sidebar counts follow. A failed request
 * leaves the badges off rather than replacing the sidebar with an error: the
 * counts are an aid, not the content.
 */
export function useCoverage(doc: DocId, generation: number): Coverage | null {
  const [state, setState] = useState<{
    doc: DocId
    generation: number
    coverage: Coverage
  } | null>(null)

  useEffect(() => {
    if (state?.doc === doc && state.generation === generation) return
    let live = true
    const load = () => {
      void call('inventory.coverage', { doc }).then(
        (result) => {
          if (!live) return
          setState({
            doc,
            generation,
            coverage: {
              categories: index(result.categories),
              subCategories: index(result.sub_categories),
              scripts: index(result.scripts),
              blocks: index(result.blocks),
              totalGlyphs: result.total_glyphs,
            },
          })
        },
        () => {
          if (live) setState({ doc, generation, coverage: EMPTY })
        },
      )
    }
    if (state === null || state.doc !== doc) {
      load()
      return () => {
        live = false
      }
    }
    const timer = setTimeout(load, COVERAGE_REFRESH_MS)
    return () => {
      live = false
      clearTimeout(timer)
    }
  }, [doc, generation, state])

  return state?.doc === doc ? state.coverage : null
}
