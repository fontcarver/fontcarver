import { useEffect, useState } from 'react'
import { call, type VariableProof, type VariableProofParams } from '../../api'

interface Response {
  key: string
  proof: VariableProof | null
  error: string | null
}

/** Debounced, read-only proof requests with revision and response-order guards. */
export function useVariableProof(
  request: VariableProofParams | null,
  generation: number,
) {
  const serialized = request ? JSON.stringify(request) : null
  const key = `${generation}:${serialized}`
  const [response, setResponse] = useState<Response | null>(null)
  useEffect(() => {
    if (serialized === null) return
    let cancelled = false
    const timer = setTimeout(() => {
      const params = JSON.parse(serialized) as VariableProofParams
      void call('text.variable_preview', params).then(
        (proof) => {
          if (!cancelled && proof.generation === generation)
            setResponse({ key, proof, error: null })
        },
        (error: unknown) => {
          if (!cancelled)
            setResponse({ key, proof: null, error: String(error) })
        },
      )
    }, 150)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [generation, key, serialized])
  const current = serialized !== null && response?.key === key ? response : null
  return { proof: current?.proof ?? null, error: current?.error ?? null }
}
