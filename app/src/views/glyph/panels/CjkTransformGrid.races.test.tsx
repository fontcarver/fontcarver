import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, expect, it, vi } from 'vitest'

import { setTransport, type Component } from '../../../api'
import * as editorRegistry from '../../../editor/activeEditor'
import {
  holdDocumentEdits,
  noteDocumentEdit,
} from '../../../editor/editSession'
import {
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
} from '../../../test/fontFixtures'
import {
  createMockTransport,
  noChange,
  type MockHandlers,
} from '../../../test/mockTransport'
import { buildScene } from '../glyphScene'
import { CjkTransformGrid } from './CjkTransformGrid'

const target = { doc: 1, glyph: 'uni6797', layer: 'm1' }
const component: Component = {
  base: 'uni6728',
  transform: [1, 0, 0, 1, 35, 0],
  anchor: 'top',
  auto_align: true,
  use_my_metrics: false,
  round_to_grid: true,
  smart: { Width: 60 },
  point_match: [2, 3],
}
const scene = buildScene({
  detail: makeGlyphDetail(makeGlyph({ name: target.glyph })),
  cell: null,
  master: makeMaster(),
  masterId: 'm1',
  unitsPerEm: 1000,
})

function outlineOf(value: Component = component) {
  return makeLayerOutline(makeLayer(), {
    components: [{ component: value, path: '', bbox: null }],
  })
}

function deferred() {
  let resolve: () => void = () => undefined
  const promise = new Promise<void>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

async function show(handlers: MockHandlers = {}) {
  const gate = deferred()
  const settle = vi.fn(() => gate.promise)
  const editor = { settle } as unknown as NonNullable<
    ReturnType<typeof editorRegistry.activeEditor>
  >
  const registry = vi
    .spyOn(editorRegistry, 'activeEditor')
    .mockReturnValue(editor)
  const mock = createMockTransport({
    'glyphs.outline': () => outlineOf(),
    'doc.apply': () => noChange([42]),
    ...handlers,
  })
  setTransport(mock)
  const reload = vi.fn()
  const props = { target, gid: 42, scene, outline: outlineOf(), reload }
  const view = render(<CjkTransformGrid {...props} />)
  const input = screen.getByRole('textbox', {
    name: 'X for component 1 (uni6728)',
  })
  const user = userEvent.setup()
  await user.clear(input)
  await user.type(input, '420{Enter}')
  await user.click(screen.getByRole('button', { name: 'Apply transforms' }))
  expect(settle).toHaveBeenCalledOnce()
  const applications = () =>
    mock.calls.filter(({ method }) => method === 'doc.apply')
  expect(applications()).toHaveLength(0)
  const finish = async () => {
    await act(async () => {
      gate.resolve()
      await gate.promise
    })
  }
  return { view, props, mock, reload, registry, applications, finish }
}

afterEach(() => {
  setTransport(undefined)
  vi.restoreAllMocks()
})

// The base replacement case was reproduced independently by the row auditor:
// before the fix the old closure wrote uni6728 over the new uni53E3 at index 0.
it.each([
  ['base replacement', { ...component, base: 'uni53E3' }],
  [
    'metadata-only replacement',
    { ...component, anchor: null, smart: { Width: 80 } },
  ],
] as const)(
  'refuses stale drafts after %s while settling',
  async (_, replacement) => {
    const state = await show()
    state.view.rerender(
      <CjkTransformGrid {...state.props} outline={outlineOf(replacement)} />,
    )
    await state.finish()
    await waitFor(() => expect(state.reload).toHaveBeenCalledOnce())
    expect(state.applications()).toHaveLength(0)
    expect(
      screen.getByRole('button', { name: 'Apply transforms' }),
    ).toBeDisabled()
  },
)

it('checks the kernel when React has not rendered the settled component change', async () => {
  const state = await show({
    'glyphs.outline': () => outlineOf({ ...component, auto_align: false }),
  })
  await state.finish()
  await waitFor(() => expect(state.reload).toHaveBeenCalledOnce())
  expect(state.applications()).toHaveLength(0)
  expect(
    state.mock.calls.some(({ method }) => method === 'glyphs.outline'),
  ).toBe(true)
})

it('retains a legitimate pending edit as exactly one batch after settling', async () => {
  const state = await show()
  await state.finish()
  await waitFor(() => expect(state.reload).toHaveBeenCalledOnce())
  expect(state.applications()).toEqual([
    {
      method: 'doc.apply',
      params: {
        doc: target.doc,
        label: 'Transform CJK components',
        command: {
          Batch: [
            {
              SetComponent: {
                glyph: target.glyph,
                layer: target.layer,
                index: 0,
                component: { ...component, transform: [1, 0, 0, 1, 420, 0] },
              },
            },
          ],
        },
      },
    },
  ])
})

it.each(['unmount', 'editor', 'target'] as const)(
  'does not dispatch or reload after changing %s',
  async (kind) => {
    const state = await show()
    if (kind === 'unmount') state.view.unmount()
    if (kind === 'editor') state.registry.mockReturnValue(null)
    if (kind === 'target')
      state.view.rerender(
        <CjkTransformGrid {...state.props} target={{ ...target, doc: 2 }} />,
      )
    await state.finish()
    expect(state.applications()).toHaveLength(0)
    expect(state.reload).not.toHaveBeenCalled()
  },
)

it('reads only after a document barrier closes and refuses an intervening kernel change', async () => {
  const barrier = deferred()
  const closing = holdDocumentEdits(target.doc, barrier.promise)
  const state = await show({
    'glyphs.outline': () => outlineOf({ ...component, point_match: null }),
  })
  await state.finish()
  expect(state.mock.calls).toHaveLength(0)
  await act(async () => {
    barrier.resolve()
    await closing
  })
  await waitFor(() => expect(state.reload).toHaveBeenCalledOnce())
  expect(state.applications()).toHaveLength(0)
})

it('does not dispatch if a newer edit starts while the source query is pending', async () => {
  const read = deferred()
  const state = await show({
    'glyphs.outline': async () => {
      await read.promise
      return outlineOf()
    },
  })
  await state.finish()
  noteDocumentEdit(target.doc)
  await act(async () => {
    read.resolve()
  })
  await waitFor(() => expect(state.reload).toHaveBeenCalledOnce())
  expect(state.applications()).toHaveLength(0)
})
