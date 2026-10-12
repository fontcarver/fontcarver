import { useEffect, useState } from 'react'

interface LayoutAnswer<T> {
  identity: string
  value?: T
  failed: boolean
}

/** Keeps asynchronous lookup pages and outlines tied to their request identity. */
export function useLayoutQuery<T>(identity: string, load: () => Promise<T>) {
  const [answer, setAnswer] = useState<LayoutAnswer<T> | null>(null)
  useEffect(() => {
    let cancelled = false
    void load().then(
      (value) => {
        if (!cancelled) setAnswer({ identity, value, failed: false })
      },
      () => {
        if (!cancelled) setAnswer({ identity, failed: true })
      },
    )
    return () => {
      cancelled = true
    }
    // The identity contains all request arguments and the refresh revision.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [identity])
  return answer?.identity === identity ? answer : null
}
