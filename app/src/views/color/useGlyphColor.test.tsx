import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { setTransport, type ColorState, type ColorPreview } from '../../api'
import { cellFor } from '../../test/glyphFixtures'
import { createMockTransport } from '../../test/mockTransport'
import { useGlyphColors } from './useGlyphColor'

const source: ColorState = {
  generation: 0,
  status: 'Editable',
  units_per_em: 1000,
  import_error: null,
  color: {
    palette_version: 0,
    entry_labels: [],
    palettes: [
      {
        colors: [{ red: 240, green: 20, blue: 10, alpha: 255 }],
        label: null,
        flags: 0,
      },
    ],
    glyphs: { A: [{ glyph: 'A', palette_index: 0 }] },
  },
}
const preview: ColorPreview = {
  generation: 0,
  width: 500,
  units_per_em: 1000,
  layers: [
    {
      glyph: 'A',
      path: 'M0 0L100 0L100 100Z',
      color: { red: 240, green: 20, blue: 10, alpha: 255 },
      bounds: [0, 0, 100, 100],
    },
  ],
}
const cells = new Map([[1, { ...cellFor(0), gid: 1, name: 'A' }]])
afterEach(() => {
  setTransport(undefined)
})

describe('shared color loading', () => {
  it('shares state and visible glyph loads, uses selected master, and reloads after document changes', async () => {
    const transport = createMockTransport({
      'color.state': () => source,
      'color.preview': () => preview,
    })
    setTransport(transport)
    const { result, rerender, unmount } = renderHook(
      ({ generation }) => [
        useGlyphColors(51, 'bold', generation, cells),
        useGlyphColors(51, 'bold', generation, cells),
      ],
      { initialProps: { generation: 0 } },
    )
    await vi.waitFor(() => expect(result.current[0]?.size).toBe(1))
    expect(result.current[0]?.get(1)).toBe(result.current[1]?.get(1))
    expect(
      transport.calls.filter((entry) => entry.method === 'color.state'),
    ).toHaveLength(1)
    expect(
      transport.calls.filter((entry) => entry.method === 'color.preview'),
    ).toEqual([
      {
        method: 'color.preview',
        params: expect.objectContaining({ master: 'bold', glyph: 'A' }),
      },
    ])
    rerender({ generation: 1 })
    expect(result.current[0]?.size).toBe(0)
    await vi.waitFor(() => expect(result.current[0]?.size).toBe(1))
    expect(
      transport.calls.filter((entry) => entry.method === 'color.state'),
    ).toHaveLength(2)
    unmount()
  })

  it('drops stale loads from the previous master without exposing old artwork', async () => {
    let finish!: (value: ColorPreview) => void
    const transport = createMockTransport({
      'color.state': () => source,
      'color.preview': ({ master }) =>
        master === 'old'
          ? new Promise((resolve) => {
              finish = resolve
            })
          : preview,
    })
    setTransport(transport)
    const { result, rerender } = renderHook(
      ({ master }) => useGlyphColors(52, master, 0, cells),
      { initialProps: { master: 'old' } },
    )
    await vi.waitFor(() => expect(finish).toBeDefined())
    rerender({ master: 'new' })
    await vi.waitFor(() => expect(result.current.size).toBe(1))
    const current = result.current.get(1)
    await act(async () => {
      finish(preview)
    })
    expect(result.current.get(1)).toBe(current)
  })

  it('renders imported preview data without taking editable ownership', async () => {
    const transport = createMockTransport({
      'color.state': () => ({
        ...source,
        status: 'Imported',
        color: null,
        preview_color: source.color ?? undefined,
      }),
      'color.preview': () => preview,
    })
    setTransport(transport)
    const { result } = renderHook(() => useGlyphColors(53, null, 0, cells))
    await vi.waitFor(() => expect(result.current.size).toBe(1))
    expect(
      transport.calls.some(
        (entry) =>
          entry.method === 'color.decode_imported' ||
          entry.method === 'color.apply',
      ),
    ).toBe(false)
  })
})
