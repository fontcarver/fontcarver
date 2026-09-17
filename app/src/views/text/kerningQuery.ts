import { useEffect, useState } from 'react'
import {
  call,
  type DocId,
  type KerningResolution,
  type KerningStatus,
} from '../../api'
import type { KerningPairNames } from './kerningModel'

interface Query {
  key: string
  status: KerningStatus | null
  resolution: KerningResolution | null
  failed: boolean
}

/** Reads one concrete pair and discards replies from older document contexts. */
export function useStoredKerning(
  doc: DocId,
  master: string | null,
  generation: number,
  pair: KerningPairNames,
  revision: number,
) {
  const key = JSON.stringify([doc, master, generation, pair.left, pair.right])
  const [query, setQuery] = useState<Query | null>(null)
  useEffect(() => {
    let current = true
    const request =
      master && pair.left && pair.right
        ? call('kerning.resolve', {
            doc,
            master,
            left: pair.left,
            right: pair.right,
          }).then((result) => ({
            key,
            status: result.status,
            resolution: result,
            failed: false,
          }))
        : call('kerning.status', { doc }).then((status) => ({
            key,
            status,
            resolution: null,
            failed: false,
          }))
    void request
      .then((result) => {
        if (current) setQuery(result)
      })
      .catch(() => {
        if (current)
          setQuery({ key, status: null, resolution: null, failed: true })
      })
    return () => {
      current = false
    }
  }, [doc, master, generation, pair.left, pair.right, key, revision])
  return query?.key === key ? query : null
}
