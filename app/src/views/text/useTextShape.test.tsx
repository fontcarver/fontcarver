import { StrictMode } from 'react'
import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { setTransport } from '../../api'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import { createMockTransport } from '../../test/mockTransport'
import { useTextShape } from './useTextShape'
import type { TextRequestContext } from './textLayout'

const context: TextRequestContext = {
  doc: 1,
  generation: 0,
  unitsPerEm: 1000,
  master: 'regular',
  text: 'A',
  features: [],
  direction: 'ltr',
  script: null,
  language: null,
}
const empty = {
  glyphs: [],
  names: [],
  units_per_em: 1000,
  size: null,
  stale: false,
  warnings: [],
}
afterEach(() => {
  vi.useRealTimers()
  setTransport(undefined)
  useEditorStore.setState(initialEditorState())
})

describe('React shaping integration', () => {
  it('hides old text/master context during the replacement debounce', async () => {
    vi.useFakeTimers()
    setTransport(createMockTransport({ 'text.shape': () => empty }))
    const { result, rerender } = renderHook(
      ({ request }) => useTextShape(request),
      { initialProps: { request: context }, wrapper: StrictMode },
    )
    await act(async () => {
      await vi.advanceTimersByTimeAsync(150)
    })
    expect(result.current?.context.text).toBe('A')
    rerender({ request: { ...context, text: 'B', master: 'bold' } })
    expect(result.current).toBeNull()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(150)
    })
    expect(result.current?.context.master).toBe('bold')
    expect(result.current?.context.text).toBe('B')
  })
  it('hides old script and language geometry during the replacement debounce', async () => {
    vi.useFakeTimers()
    const transport = createMockTransport({ 'text.shape': () => empty })
    setTransport(transport)
    const { result, rerender } = renderHook(
      ({ request }) => useTextShape(request),
      { initialProps: { request: context } },
    )
    await act(async () => {
      await vi.advanceTimersByTimeAsync(150)
    })
    expect(result.current).not.toBeNull()
    rerender({ request: { ...context, script: 'Arab', language: 'ar' } })
    expect(result.current).toBeNull()
    await act(async () => {
      await vi.advanceTimersByTimeAsync(150)
    })
    expect(transport.calls.at(-1)?.params).toMatchObject({
      script: 'Arab',
      language: 'ar',
    })
  })
  it('polls stale to fresh without an edit and suspends requests during gestures', async () => {
    vi.useFakeTimers()
    let stale = true
    const transport = createMockTransport({
      'text.shape': () => ({ ...empty, stale }),
    })
    setTransport(transport)
    const { result } = renderHook(() => useTextShape(context))
    await act(async () => {
      await vi.advanceTimersByTimeAsync(150)
    })
    expect(result.current?.stale).toBe(true)
    act(() => useEditorStore.getState().setEditing(true))
    const count = transport.calls.length
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000)
    })
    expect(transport.calls).toHaveLength(count)
    stale = false
    act(() => useEditorStore.getState().setEditing(false))
    await act(async () => {
      await vi.advanceTimersByTimeAsync(150)
    })
    expect(result.current?.stale).toBe(false)
  })
})
