import { renderHook, waitFor } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'

import { setTransport, type Instance } from '../../api'
import { makeSummary } from '../../test/mockTransport'
import { createMockTransport } from '../../test/mockTransport'
import type { PreviewGlyph } from './presentationTypes'
import { initialTextState } from './textState'
import { usePreviewInstance } from './usePreviewInstance'

const documentGlyphs: PreviewGlyph[] = [
  { path: 'M0 0L100 0L100 100Z', x: 0, y: 0, xAdvance: 600, yAdvance: 0 },
]

function variable() {
  return makeSummary({
    masters: [
      { id: 'm1', name: 'Regular' },
      { id: 'm2', name: 'Bold' },
    ],
    axes: [
      {
        tag: 'wght',
        name: 'Weight',
        min: 100,
        default: 400,
        max: 900,
        hidden: false,
      },
    ],
  })
}

function makeInstance(name: string): Instance {
  return {
    name,
    location: {},
    style_name: name,
    family_name: null,
    active: true,
    is_variable_named_instance: true,
    user_data: null,
  }
}

function transport(instances = [{ name: 'SemiBold' }]) {
  return createMockTransport({
    'variation.state': () => ({
      generation: 0,
      axes: [],
      masters: [],
      instances: instances.map((instance) => makeInstance(instance.name)),
      first_glyph: 'A',
    }),
    'text.variable_preview': () => ({
      generation: 0,
      location: { wght: 600 },
      glyphs: [
        {
          name: 'A',
          path: 'M0 0L200 0L200 200Z',
          x: 0,
          y: 0,
          x_advance: 700,
          y_advance: 0,
        },
      ],
      units_per_em: 1000,
      warnings: [],
    }),
  })
}

function render(state: Parameters<typeof usePreviewInstance>[0]['state']) {
  return renderHook(() =>
    usePreviewInstance({
      summary: variable(),
      state,
      generation: 0,
      documentGlyphs,
      documentActiveGlyph: documentGlyphs[0]!,
    }),
  )
}

afterEach(() => {
  setTransport(undefined)
  vi.restoreAllMocks()
})

it('offers every master and every named instance', async () => {
  setTransport(transport([{ name: 'SemiBold' }, { name: 'Black' }]))
  const { result } = render(initialTextState('m1', 'A'))
  await waitFor(() => expect(result.current.instances).toHaveLength(4))
  expect(result.current.instances).toEqual([
    { id: 'master:m1', name: 'Regular', kind: 'master' },
    { id: 'master:m2', name: 'Bold', kind: 'master' },
    { id: 'instance:0', name: 'SemiBold', kind: 'instance' },
    { id: 'instance:1', name: 'Black', kind: 'instance' },
  ])
  // Without a chosen instance the strip keeps the document's own outlines.
  expect(result.current.glyphs).toBe(documentGlyphs)
  expect(result.current.status).toBeNull()
})

it('renders the interpolated outlines of a chosen named instance', async () => {
  const mock = transport()
  setTransport(mock)
  const { result } = render({
    ...initialTextState('m1', 'A'),
    previewInstance: 0,
  })
  // Until the design space is read and the proof answers, the strip is not
  // claimed to be ready for that instance.
  expect(result.current.status).not.toBe('ready')
  await waitFor(() => expect(result.current.status).toBe('ready'))
  expect(result.current.glyphs).toEqual([
    { path: 'M0 0L200 0L200 200Z', x: 0, y: 0, xAdvance: 700, yAdvance: 0 },
  ])
  expect(result.current.activeGlyph).toEqual(result.current.glyphs[0])
  const request = mock.calls.find(
    (call) => call.method === 'text.variable_preview',
  )
  expect(request?.params).toMatchObject({ instance: 0, text: 'A' })
})

it('falls back to the document outlines when the instance no longer exists', async () => {
  setTransport(transport([]))
  const { result } = render({
    ...initialTextState('m1', 'A'),
    previewInstance: 3,
  })
  await waitFor(() => expect(result.current.instances).toHaveLength(2))
  expect(result.current.glyphs).toBe(documentGlyphs)
  expect(result.current.status).toBeNull()
})

it('reports a refused proof instead of showing stale geometry', async () => {
  setTransport(
    createMockTransport({
      'variation.state': () => ({
        generation: 0,
        axes: [],
        masters: [],
        instances: [makeInstance('SemiBold')],
        first_glyph: 'A',
      }),
      'text.variable_preview': () => {
        throw new Error('instance no longer exists')
      },
    }),
  )
  const { result } = render({
    ...initialTextState('m1', 'A'),
    previewInstance: 0,
  })
  await waitFor(() => expect(result.current.status).toBe('compile-failed'))
  expect(result.current.glyphs).toEqual([])
})

it('asks for nothing when the font has no axes', () => {
  const mock = createMockTransport({})
  setTransport(mock)
  renderHook(() =>
    usePreviewInstance({
      summary: makeSummary(),
      state: initialTextState('m1', 'A'),
      generation: 0,
      documentGlyphs,
      documentActiveGlyph: null,
    }),
  )
  expect(mock.calls).toEqual([])
})
