import { useEffect, useState } from 'react'

interface Answer<T> {
  identity: string
  value?: T
  failed: boolean
}

/** Publishes bounded query results only for the identity that requested them. */
export function useKerningGroupQuery<T>(
  identity: string,
  load: () => Promise<T>,
) {
  const [answer, setAnswer] = useState<Answer<T> | null>(null)
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
    // The caller's identity includes every request argument and refresh stamp.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [identity])
  return answer?.identity === identity ? answer : null
}
