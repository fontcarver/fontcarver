import { call, type Component, type DocId, type GlyphCell } from '../../../api'
import { emptyFilter } from '../../../stores/documentStore'
import { layoutIds } from './cjkConstruction'
import { newComponent } from './panelCommands'

/** A component and its resolved source outline for the construction preview. */
export interface CjkResolvedComponent {
  component: Component
  cell: GlyphCell
}

/** A specific leaf that cannot safely be resolved in the current document. */
export class CjkResolveError extends Error {
  constructor(
    readonly code: 'missing' | 'ambiguous' | 'self' | 'empty' | 'width',
    readonly token: string,
  ) {
    super(code)
  }
}

/** Resolve bounded IDS leaves without mutating the document. */
export async function resolveCjkIds(
  doc: DocId,
  master: string | null,
  glyph: string,
  text: string,
  width: number,
  upem: number,
  descender: number,
): Promise<CjkResolvedComponent[]> {
  const leaves = layoutIds(text, width, upem, descender)
  const cells = new Map<string, GlyphCell>()
  await Promise.all(
    [...new Set(leaves.map(({ token }) => token))].map(async (token) => {
      const named = token.startsWith('[')
      const name = named ? token.slice(1, -1) : ''
      const unicode = token.codePointAt(0) ?? 0
      const result = await call('glyphs.list', {
        doc,
        master,
        filter: {
          ...emptyFilter(),
          query: named ? name : null,
          unicode_ranges: named ? [] : [[unicode, unicode]],
        },
        sort: 'Order',
        offset: 0,
        limit: 2000,
        include_paths: true,
      })
      const matching = result.items.filter((cell) =>
        named ? cell.name === name : cell.unicodes.includes(unicode),
      )
      if (result.total > 2000 || matching.length > 1)
        throw new CjkResolveError('ambiguous', token)
      const cell = matching[0]
      if (!cell) throw new CjkResolveError('missing', token)
      if (cell.name === glyph) throw new CjkResolveError('self', token)
      if (cell.is_empty) throw new CjkResolveError('empty', token)
      if (!(cell.width > 0) || !Number.isFinite(cell.width))
        throw new CjkResolveError('width', token)
      cells.set(token, cell)
    }),
  )
  return leaves.map(({ token, transform }) => {
    const cell = cells.get(token)
    // Every token was resolved above; keep an explicit failure at this boundary.
    if (!cell) throw new CjkResolveError('missing', token)
    const normalized = [...transform] as Component['transform']
    normalized[0] *= width / cell.width
    normalized[1] *= width / cell.width
    return {
      cell,
      component: { ...newComponent(cell.name), transform: normalized },
    }
  })
}
