import { useEffect, useMemo, useState } from 'react'
import type { DocId, GlyphCell, GlyphId, MasterId } from '../../api'
import type { ColorGlyphPaint } from '../../canvas/colorGlyph'
import {
  onDocumentForgotten,
  selectDocView,
  useDocumentStore,
} from '../../stores/documentStore'
import { ColorGlyphCache } from './colorGlyphCache'
import { colorGlyphLoader } from './loadColorGlyph'

interface Context {
  doc: DocId
  refs: number
  cache: ColorGlyphCache
  listeners: Set<() => void>
  names: Promise<ReadonlySet<string>>
}

const contexts = new Map<string, Context>()
const EMPTY: ReadonlyMap<GlyphId, ColorGlyphPaint> = new Map()

function retainContext(
  doc: DocId,
  master: MasterId | null,
  generation: number,
): { context: Context; release(): void } {
  const identity = JSON.stringify([doc, master, generation])
  let context = contexts.get(identity)
  if (!context) {
    const listeners = new Set<() => void>()
    const load = colorGlyphLoader(doc, master, () => {
      for (const listener of listeners) listener()
    })
    context = {
      doc,
      refs: 0,
      listeners,
      names: load.names,
      cache: new ColorGlyphCache(load),
    }
    contexts.set(identity, context)
  }
  context.refs += 1
  return {
    context,
    release() {
      context.refs -= 1
      // React effect replacements can retain the same generation immediately.
      queueMicrotask(() => {
        if (context.refs !== 0 || contexts.get(identity) !== context) return
        contexts.delete(identity)
        context.cache.dispose()
      })
    },
  }
}

onDocumentForgotten((doc) => {
  for (const [key, context] of contexts) {
    if (context.doc !== doc) continue
    contexts.delete(key)
    context.cache.dispose()
  }
})

/** Load only supplied glyphs, sharing validated source, work limits and resources. */
export function useGlyphColors(
  doc: DocId,
  master: MasterId | null,
  generation: number,
  cells: ReadonlyMap<GlyphId, GlyphCell>,
): ReadonlyMap<GlyphId, ColorGlyphPaint> {
  const signature = JSON.stringify(
    [...cells].map(([gid, cell]) => [gid, cell.name]),
  )
  const identity = JSON.stringify([doc, master, generation, signature])
  const [loaded, setLoaded] = useState<{
    identity: string
    colors: ReadonlyMap<GlyphId, ColorGlyphPaint>
  }>({ identity: '', colors: EMPTY })
  useEffect(() => {
    const requested = JSON.parse(signature) as [GlyphId, string][]
    if (!requested.length) return
    const retained = retainContext(doc, master, generation)
    const { context } = retained
    let active = true
    const colors = new Map<GlyphId, ColorGlyphPaint>()
    const publish = () => {
      if (active) setLoaded({ identity, colors: new Map(colors) })
    }
    context.listeners.add(publish)
    const leases: ReturnType<ColorGlyphCache['acquire']>[] = []
    void context.names.then((names) => {
      if (!active) return
      for (const [gid, name] of requested) {
        // Monochrome cells must not consume the limited color-resource slots.
        if (!names.has(name)) continue
        const lease = context.cache.acquire(name)
        leases.push(lease)
        void lease.result.then((resource) => {
          if (!active || !resource) return
          colors.set(gid, resource)
          publish()
        })
      }
    })
    return () => {
      active = false
      context.listeners.delete(publish)
      for (const lease of leases) lease.release()
      retained.release()
    }
  }, [doc, generation, identity, master, signature])
  return loaded.identity === identity ? loaded.colors : EMPTY
}

/** Inventory cells follow the same document generation and selected master. */
export function useCellGlyphColor(
  doc: DocId,
  cell: GlyphCell | undefined,
): ColorGlyphPaint | undefined {
  const view = useDocumentStore(selectDocView(doc))
  const cells = useMemo(() => new Map(cell ? [[cell.gid, cell]] : []), [cell])
  return useGlyphColors(doc, view.master, view.generation, cells).get(
    cell?.gid ?? -1,
  )
}
