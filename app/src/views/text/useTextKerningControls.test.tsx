import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { PositionedGlyph } from './textLayout'
import { useTextKerningControls } from './useTextKerningControls'

vi.mock('./useTextKerning', async () => {
  const { useState, useCallback } = await import('react')
  return {
    useTextKerning: () => {
      const [pair, setNames] = useState({ left: '', right: '' })
      const setPair = useCallback(
        async (next: { left: string; right: string }) => {
          setNames(next)
        },
        [],
      )
      return {
        panel: { pair, setPair },
        finish: async () => undefined,
        onKerningKey: () => false,
      }
    },
  }
})

const glyphs: PositionedGlyph[] = ['A', 'V', 'T'].map((name, index) => ({
  name,
  documentGid: index + 7,
  compiledGid: index + 17,
  editable: true,
  x: index * 600,
  y: 0,
  xAdvance: 600,
  yAdvance: 0,
  sourceByte: index,
  sourceKind: 'literal',
}))
const base = {
  doc: 1,
  master: 'regular',
  generation: 1,
  text: 'AVT',
  direction: 'ltr',
  features: [] as Array<[string, boolean]>,
  glyphs,
  pending: false,
  transition: async (action: () => void) => {
    action()
  },
}

describe('shaped gap selection handover', () => {
  it('removes a misleading highlight after explicit names and can reselect the same gap', async () => {
    const hook = renderHook(() => useTextKerningControls(base))
    await act(async () => {
      hook.result.current.selectGap(0)
    })
    expect(hook.result.current.selectedGap).toBe(0)
    await act(async () => {
      await hook.result.current.panel.setPair({ left: 'A', right: 'T' })
    })
    expect(hook.result.current.selectedGap).toBeNull()
    await act(async () => {
      hook.result.current.selectGap(0)
    })
    expect(hook.result.current.panel.pair).toEqual({ left: 'A', right: 'V' })
    expect(hook.result.current.selectedGap).toBe(0)
  })

  it('drops a delayed old document selection and the earlier of two pending clicks', async () => {
    const releases: Array<() => void> = []
    const transition = (action: () => void) =>
      new Promise<void>((resolve) => {
        releases.push(() => {
          action()
          resolve()
        })
      })
    const hook = renderHook(
      ({ doc }) => useTextKerningControls({ ...base, doc, transition }),
      { initialProps: { doc: 1 } },
    )
    act(() => {
      hook.result.current.selectGap(0)
    })
    hook.rerender({ doc: 2 })
    await act(async () => {
      releases.shift()?.()
    })
    expect(hook.result.current.panel.pair.left).toBe('')
    act(() => {
      hook.result.current.selectGap(0)
      hook.result.current.selectGap(1)
    })
    await act(async () => {
      releases[1]?.()
    })
    await act(async () => {
      releases[0]?.()
    })
    expect(hook.result.current.panel.pair).toEqual({ left: 'V', right: 'T' })
    expect(hook.result.current.selectedGap).toBe(1)
  })

  it('blocks a stale canvas and explains a run consisting only of one ligature', async () => {
    const hook = renderHook(() =>
      useTextKerningControls({ ...base, pending: true }),
    )
    await act(async () => {
      hook.result.current.selectGap(0)
    })
    expect(hook.result.current.panel.pair.left).toBe('')
    const ligature = renderHook(() =>
      useTextKerningControls({
        ...base,
        text: 'fi',
        glyphs: [{ ...glyphs[0]!, name: 'fi' }],
      }),
    )
    expect(ligature.result.current.disabledReason).toContain('ligature')
  })
})
