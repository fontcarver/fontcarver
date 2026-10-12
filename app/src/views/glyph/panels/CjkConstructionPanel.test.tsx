import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import {
  setTransport,
  type ApplyParams,
  type CjkRadicalLibrary,
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
/** Radical 75 木, as the kernel's data-driven library reports it. */
const radicalLibrary: CjkRadicalLibrary = {
  unihan_version: '17.0.0',
  radicals: [
    {
      number: 75,
      kangxi: 0x2f4a,
      kangxi_glyph: null,
      forms: [{ unicode: 0x6728, strokes: 4, glyph: 'wood' }],
      members: 1,
    },
  ],
  components: [{ name: 'wood', uses: 2, unicode: 0x6728 }],
  components_truncated: false,
}

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

  it('inserts a radical the pinned data named, and a font component', async () => {
    setTransport(
      createMockTransport({
        'glyphs.list': () => ({ total: 1, items: [wood] }),
        'cjk.radicals': () => radicalLibrary,
      }),
    )
    render(<CjkConstructionPanel {...context()} />)
    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Radical library' }))
    // The entry is described by its radical number, stroke count, scalar and
    // glyph, all of which come from the kernel's Unihan-derived library.
    const radical = await screen.findByRole('button', {
      name: /Insert Radical 75 · 4 strokes · U\+6728 · wood · 1 glyph in this font/,
    })
    await user.click(radical)
    const ids = screen.getByLabelText('Ideographic description sequence')
    expect(ids).toHaveValue('[wood]')

    // The second tab lists what the font already uses as components.
    await user.click(screen.getByRole('tab', { name: 'Font components' }))
    await user.click(
      await screen.findByRole('button', {
        name: /Insert U\+6728 · wood · 2 component uses/,
      }),
    )
    expect(ids).toHaveValue('[wood][wood]')
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
