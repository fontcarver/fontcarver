import { useEffect, useState } from 'react'
import type { DocId, KerningPairs } from '../../api'
import { loadKerningTable } from './kerningTableData'

/** Lazy, cancellable table reads; a changed identity immediately hides old data. */
export function useKerningTable(
  doc: DocId,
  master: string | null,
  vertical: boolean,
  generation: number,
  open: boolean,
  revision: number,
) {
  const identity = JSON.stringify([
    doc,
    master,
    vertical,
    generation,
    open,
    revision,
  ])
  const [answer, setAnswer] = useState<{
    identity: string
    data: KerningPairs | null
    failed: boolean
  } | null>(null)
  useEffect(() => {
    if (!open || !master) return
    let current = true
    void loadKerningTable(doc, master, vertical, () => current).then(
      (data) => {
        if (current) setAnswer({ identity, data, failed: false })
      },
      () => {
        if (current) setAnswer({ identity, data: null, failed: true })
      },
    )
    return () => {
      current = false
    }
    // Identity contains every query argument and the refresh counter.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [identity])
  return answer?.identity === identity ? answer : null
}
