import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import {
  setTransport,
  type ApplyParams,
  type GlyphListResult,
} from '../../../api'
import { setCommandErrorReporter } from '../../../shell/commands'
import {
  initialDocumentState,
  newDocView,
  useDocumentStore,
} from '../../../stores/documentStore'
import {
  makeCell,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
} from '../../../test/fontFixtures'
import { createMockTransport, noChange } from '../../../test/mockTransport'
import { buildScene } from '../glyphScene'
import { CjkConstructionPanel } from './CjkConstructionPanel'
import type { PanelContext } from './panelContext'

const wood = makeCell({
  name: 'wood',
  unicodes: [0x6728],
  width: 1000,
  path: 'M100 0L900 0L900 800L100 800Z',
  bbox: [100, 0, 900, 800],
})
function context(): PanelContext {
  const layer = makeLayer({ width: 1000 })
  const glyph = makeGlyph({ name: 'forest', layers: [layer] })
  return {
    target: { doc: 1, glyph: 'forest', layer: layer.id },
    gid: 42,
    glyph,
    cell: makeCell(),
    scene: buildScene({
      detail: makeGlyphDetail(glyph),
      cell: makeCell(),
      master: makeMaster(),
      masterId: 'm1',
      unitsPerEm: 1000,
    }),
    outline: makeLayerOutline(layer),
    master: 'm1',
    masterGuides: [],
    reload: vi.fn(),
  }
}

beforeEach(() => useDocumentStore.setState(initialDocumentState()))
afterEach(() => {
  setTransport(undefined)
  setCommandErrorReporter(() => undefined)
})

describe('CJK construction panel', () => {
  it('previews and appends one atomic command without replacing existing content', async () => {
    const transport = createMockTransport({
      'glyphs.list': () => ({ total: 1, items: [wood] }),
      'doc.apply': () => noChange([42]),
    })
    setTransport(transport)
    const props = context()
    render(<CjkConstructionPanel {...props} />)
    const user = userEvent.setup()
    fireEvent.change(
      screen.getByLabelText('Ideographic description sequence'),
      { target: { value: '⿰木木' } },
    )
    await user.click(screen.getByRole('button', { name: 'Preview IDS' }))
    expect(
      await screen.findByRole('img', { name: 'IDS component preview' }),
    ).toBeVisible()
    await user.click(screen.getByRole('button', { name: 'Append components' }))
    await waitFor(() => expect(props.reload).toHaveBeenCalledOnce())
    const request = transport.calls.find(({ method }) => method === 'doc.apply')
      ?.params as ApplyParams
    expect(request.command).toMatchObject({
      Batch: [
        {
          AddComponent: {
            glyph: 'forest',
            layer: 'layer-1',
            component: { base: 'wood', transform: [0.5, 0, 0, 1, 0, 0] },
          },
        },
        {
          AddComponent: {
            glyph: 'forest',
            layer: 'layer-1',
            component: { base: 'wood', transform: [0.5, 0, 0, 1, 500, 0] },
          },
        },
      ],
    })
    expect(
      screen.getByRole('button', { name: 'Append components' }),
    ).toBeDisabled()
  })

  it('inserts a named radical from the current document library', async () => {
    setTransport(
      createMockTransport({
        'glyphs.list': () => ({ total: 1, items: [wood] }),
      }),
    )
    render(<CjkConstructionPanel {...context()} />)
    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Radical library' }))
    await user.click(
      await screen.findByRole('button', { name: 'Insert wood into IDS' }),
    )
    expect(
      screen.getByLabelText('Ideographic description sequence'),
    ).toHaveValue('[wood]')
  })

  it('reports missing source glyphs and never enables mutation', async () => {
    const transport = createMockTransport({
      'glyphs.list': () => ({ total: 0, items: [] }),
    })
    setTransport(transport)
    render(<CjkConstructionPanel {...context()} />)
    fireEvent.change(
      screen.getByLabelText('Ideographic description sequence'),
      { target: { value: '木' } },
    )
    await userEvent.click(screen.getByRole('button', { name: 'Preview IDS' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'No glyph matches 木',
    )
    expect(
      screen.getByRole('button', { name: 'Append components' }),
    ).toBeDisabled()
    expect(transport.calls.some(({ method }) => method === 'doc.apply')).toBe(
      false,
    )
  })

  it('drops a pending preview when the document generation changes', async () => {
    let answer: (result: GlyphListResult) => void = () => undefined
    const pending = new Promise<GlyphListResult>((resolve) => {
      answer = resolve
    })
    setTransport(createMockTransport({ 'glyphs.list': () => pending }))
    render(<CjkConstructionPanel {...context()} />)
    fireEvent.change(
      screen.getByLabelText('Ideographic description sequence'),
      { target: { value: '木' } },
    )
    await userEvent.click(screen.getByRole('button', { name: 'Preview IDS' }))
    act(() =>
      useDocumentStore.setState({
        byDoc: { 1: { ...newDocView(), generation: 1 } },
      }),
    )
    await act(async () => {
      answer({ total: 1, items: [wood] })
      await pending
    })
    expect(screen.queryByRole('img')).not.toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Append components' }),
    ).toBeDisabled()
  })

  it('reports a refused command and reloads the actual layer', async () => {
    const report = vi.fn()
    setCommandErrorReporter(report)
    setTransport(
      createMockTransport({
        'glyphs.list': () => ({ total: 1, items: [wood] }),
        'doc.apply': () => {
          throw new Error('component cycle')
        },
      }),
    )
    const props = context()
    render(<CjkConstructionPanel {...props} />)
    fireEvent.change(
      screen.getByLabelText('Ideographic description sequence'),
      { target: { value: '木' } },
    )
    await userEvent.click(screen.getByRole('button', { name: 'Preview IDS' }))
    await userEvent.click(
      screen.getByRole('button', { name: 'Append components' }),
    )
    await waitFor(() => expect(report).toHaveBeenCalledOnce())
    expect(props.reload).toHaveBeenCalledOnce()
    expect(screen.getByRole('alert')).toHaveTextContent(
      'Could not add these components',
    )
  })
})
