import { renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type KerningBoundariesParams } from '../../api'
import { createMockTransport } from '../../test/mockTransport'
import type { TextGap } from './textGaps'
import { boundaryPairs, useKerningExceptions } from './useKerningExceptions'

function gap(index: number, left: string, right: string, reason = null) {
  return { index, left, right, x: index * 500, reason } as TextGap
}

afterEach(() => setTransport(undefined))

describe('boundaryPairs', () => {
  it('asks only about boundaries that identify an editable pair', () => {
    expect(
      boundaryPairs([
        gap(0, 'A', 'V'),
        { ...gap(1, 'V', 'A'), reason: 'cluster' } as TextGap,
        { ...gap(2, 'A', 'V'), left: null } as TextGap,
        gap(3, 'T', 'o'),
      ]),
    ).toEqual([
      { index: 0, pair: ['A', 'V'] },
      { index: 3, pair: ['T', 'o'] },
    ])
  })

  it('stays bounded however long the run is', () => {
    const many = Array.from({ length: 40 }, (_, index) => gap(index, 'A', 'V'))
    expect(boundaryPairs(many, 8)).toHaveLength(8)
  })
})

describe('useKerningExceptions', () => {
  const options = {
    doc: 1,
    master: 'm1',
    generation: 2,
    gaps: [gap(0, 'A', 'V'), gap(1, 'V', 'A')],
    vertical: false,
    enabled: true,
  }

  it('marks the boundaries the kernel reports as exceptions', async () => {
    const boundaries = vi.fn((params: KerningBoundariesParams) => {
      seen.push(params)
      return {
        status: { generation: 2, editable: true, reason: null },
        items: [
          { value: -40, exception: true },
          { value: 0, exception: false },
        ],
      }
    })
    const seen: KerningBoundariesParams[] = []
    setTransport(createMockTransport({ 'kerning.boundaries': boundaries }))
    const hook = renderHook(() => useKerningExceptions(options))
    await vi.waitFor(() => expect(hook.result.current.has(0)).toBe(true))
    expect(hook.result.current.has(1)).toBe(false)
    expect(boundaries).toHaveBeenCalledOnce()
    // The request carries the pairs in run order, not the gap indexes.
    expect(seen[0]).toMatchObject({
      pairs: [
        ['A', 'V'],
        ['V', 'A'],
      ],
      vertical: false,
    })
    hook.unmount()
  })

  it('marks nothing when the kernel refuses and never asks while disabled', async () => {
    const boundaries = vi.fn(() => {
      throw new Error('unavailable')
    })
    setTransport(createMockTransport({ 'kerning.boundaries': boundaries }))
    const hook = renderHook(
      (props: typeof options) => useKerningExceptions(props),
      { initialProps: { ...options, enabled: false } },
    )
    expect(hook.result.current.size).toBe(0)
    expect(boundaries).not.toHaveBeenCalled()
    hook.rerender(options)
    await vi.waitFor(() => expect(boundaries).toHaveBeenCalled())
    expect(hook.result.current.size).toBe(0)
    hook.unmount()
  })
})
