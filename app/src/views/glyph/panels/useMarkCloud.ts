import { useEffect, useState } from 'react'

import {
  call,
  onDocumentEvent,
  type DocId,
  type GlyphCell,
  type GlyphDetail,
  type GlyphId,
} from '../../../api'
import { emptyFilter } from '../../../stores/documentStore'

/** The maximum mark candidates loaded for one preview page. */
export const MARK_CLOUD_PAGE_SIZE = 24
const DETAIL_CONCURRENCY = 4
let activeDetails = 0
const waitingDetails: (() => void)[] = []

// Ignored stale transport calls retain their shared slot until they settle.
async function detailSlot<T>(operation: () => Promise<T>): Promise<T> {
  if (activeDetails >= DETAIL_CONCURRENCY) {
    await new Promise<void>((resolve) => waitingDetails.push(resolve))
  } else {
    activeDetails += 1
  }
  try {
    return await operation()
  } finally {
    const next = waitingDetails.shift()
    if (next) next()
    else activeDetails -= 1
  }
}

/** A candidate's metadata and exact-master cell geometry. */
export interface MarkCandidate {
  cell: GlyphCell
  detail: GlyphDetail
}

interface Loaded {
  key: string
  total: number
  count: number
  candidates: MarkCandidate[]
  failed: boolean
  partial: boolean
}

/** Loads a bounded mark page, abandoning subsequent work when its owner changes. */
export function useMarkCloud(
  doc: DocId,
  gid: GlyphId,
  master: string,
  page: number,
  enabled: boolean,
  changePage: (page: number) => void,
) {
  const [generation, setGeneration] = useState(0)
  const [loaded, setLoaded] = useState<Loaded | null>(null)
  const key = JSON.stringify([doc, gid, master, page, generation])

  useEffect(() => {
    if (!enabled) return
    let cancelled = false
    async function load() {
      try {
        const result = await call('glyphs.list', {
          doc,
          master,
          filter: { ...emptyFilter(), categories: ['Mark'] },
          sort: 'Name',
          offset: page * MARK_CLOUD_PAGE_SIZE,
          limit: MARK_CLOUD_PAGE_SIZE,
          include_paths: true,
        })
        if (cancelled) return
        if (page > 0 && page * MARK_CLOUD_PAGE_SIZE >= result.total) {
          changePage(
            Math.max(0, Math.ceil(result.total / MARK_CLOUD_PAGE_SIZE) - 1),
          )
          return
        }
        const candidates: (MarkCandidate | null)[] = result.items.map(
          () => null,
        )
        let cursor = 0
        let partial = false
        async function worker() {
          while (!cancelled && cursor < result.items.length) {
            const index = cursor++
            const cell = result.items[index]
            if (!cell || cell.gid === gid) continue
            try {
              const detail = await detailSlot(async () =>
                cancelled
                  ? null
                  : call('glyphs.detail', { doc, gid: cell.gid }),
              )
              if (!cancelled && detail) candidates[index] = { cell, detail }
            } catch {
              partial = true
            }
          }
        }
        await Promise.all(Array.from({ length: DETAIL_CONCURRENCY }, worker))
        if (!cancelled)
          setLoaded({
            key,
            total: result.total,
            count: result.items.length,
            candidates: candidates.filter(
              (candidate): candidate is MarkCandidate => candidate !== null,
            ),
            failed: false,
            partial,
          })
      } catch {
        if (!cancelled)
          setLoaded({
            key,
            total: 0,
            count: 0,
            candidates: [],
            failed: true,
            partial: false,
          })
      }
    }
    void load()
    return () => {
      cancelled = true
    }
  }, [doc, gid, master, page, enabled, key, changePage])

  useEffect(() => {
    if (!enabled) return
    return onDocumentEvent((event) => {
      if (event.doc !== doc) return
      // Base edits already update the supplied outline. Reload marks only for
      // edits elsewhere, preserving responsive base-anchor dragging.
      if (
        event.change.glyph_order ||
        event.change.masters ||
        event.change.glyphs.some((changed) => changed !== gid)
      ) {
        setGeneration((value) => value + 1)
      }
    })
  }, [doc, gid, enabled])

  const current = loaded?.key === key ? loaded : null
  return {
    candidates: current?.candidates ?? [],
    total: current?.total ?? 0,
    count: current?.count ?? 0,
    loading: enabled && current === null,
    failed: current?.failed ?? false,
    partial: current?.partial ?? false,
    retry: () => setGeneration((value) => value + 1),
  }
}
