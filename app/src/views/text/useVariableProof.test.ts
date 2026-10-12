import { act, renderHook } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import {
  setTransport,
  type VariableProof,
  type VariableProofParams,
} from '../../api'
import { createMockTransport } from '../../test/mockTransport'
import { useVariableProof } from './useVariableProof'

const request: VariableProofParams = {
  doc: 1,
  text: 'AV',
  location: { wght: 100 },
  instance: null,
  features: [],
  script: null,
  language: null,
  direction: 'ltr',
}
function proof(generation: number): VariableProof {
  return {
    generation,
    location: { wght: 100 },
    glyphs: [],
    units_per_em: 1000,
    warnings: [],
  }
}

afterEach(() => {
  setTransport(undefined)
  vi.useRealTimers()
})

it('does not request a collapsed proof and ignores old location replies', async () => {
  vi.useFakeTimers()
  const replies: ((proof: VariableProof) => void)[] = []
  const transport = createMockTransport({
    'text.variable_preview': () =>
      new Promise((resolve) => replies.push(resolve)),
  })
  setTransport(transport)
  const { result, rerender } = renderHook(
    ({ value, generation }) => useVariableProof(value, generation),
    {
      initialProps: {
        value: null as VariableProofParams | null,
        generation: 0,
      },
    },
  )
  await act(() => vi.advanceTimersByTimeAsync(200))
  expect(transport.calls).toHaveLength(0)
  rerender({ value: request, generation: 0 })
  await act(() => vi.advanceTimersByTimeAsync(200))
  rerender({ value: { ...request, location: { wght: 900 } }, generation: 1 })
  await act(() => vi.advanceTimersByTimeAsync(200))
  await act(async () => replies[0]!(proof(0)))
  expect(result.current.proof).toBeNull()
  await act(async () => replies[1]!(proof(1)))
  expect(result.current.proof?.generation).toBe(1)
  rerender({ value: null, generation: 1 })
  expect(result.current.proof).toBeNull()
})

it('rejects a response from a different document revision', async () => {
  vi.useFakeTimers()
  setTransport(createMockTransport({ 'text.variable_preview': () => proof(1) }))
  const { result } = renderHook(() => useVariableProof(request, 0))
  await act(() => vi.advanceTimersByTimeAsync(200))
  expect(result.current.proof).toBeNull()
})
