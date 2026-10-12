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
        panel: {
          pair,
          setPair,
          editable: true,
          status: { editable: true },
          setValue: vi.fn(),
          commit: vi.fn(),
          remove: vi.fn(),
          setScope: vi.fn(),
        },
        finish: async () => undefined,
        onKerningKey: useCallback(() => true, []),
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
  it('keeps the canvas keyboard callback stable across unrelated renders', () => {
    const { result, rerender } = renderHook(() => useTextKerningControls(base))
    const callback = result.current.onKerningKey
    rerender()
    expect(result.current.onKerningKey).toBe(callback)
  })

  it('disables shaping gap selection and panel mutations when kerning is off', async () => {
    const hook = renderHook(() =>
      useTextKerningControls({ ...base, kerningMode: 'off' }),
    )
    expect(hook.result.current.panel.editable).toBe(false)
    expect(hook.result.current.panel.status?.editable).toBe(false)
    expect(hook.result.current.disabledReason).toContain('disabled')
    await act(async () => {
      hook.result.current.selectGap(0)
      await hook.result.current.panel.setPair({ left: 'A', right: 'V' })
      await hook.result.current.panel.commit()
      await hook.result.current.panel.remove()
    })
    expect(hook.result.current.selectedGap).toBeNull()
    expect(hook.result.current.panel.pair).toEqual({ left: '', right: '' })
    expect(
      hook.result.current.onKerningKey({
        key: '.',
        altKey: false,
        ctrlKey: false,
        metaKey: false,
        shiftKey: false,
      }),
    ).toBe(false)
  })

  it('allows punctuation kerning in locked mode only after selecting a gap', async () => {
    const hook = renderHook(() =>
      useTextKerningControls({ ...base, kerningMode: 'locked' }),
    )
    const key = {
      key: '.',
      altKey: false,
      ctrlKey: false,
      metaKey: false,
      shiftKey: false,
    }
    expect(hook.result.current.onKerningKey(key)).toBe(false)
    await act(async () => hook.result.current.selectGap(0))
    expect(hook.result.current.onKerningKey(key)).toBe(true)
    expect(hook.result.current.panel.editable).toBe(true)
  })

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
