import { act, cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'

import { setTransport, type StyleBatchResult } from '../api'
import { useEditorStore, type ActiveGlyph } from '../stores/editorStore'
import {
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
} from '../test/fontFixtures'
import { createMockTransport } from '../test/mockTransport'
import { buildScene } from '../views/glyph/glyphScene'
import { useFilterPreview } from './filterSession'
import { PluginManager } from './PluginManager'
import { PLUGIN_STORAGE_KEY, usePluginStore } from './pluginStore'

const pluginId = 'fontcarver.translate'
const result: StyleBatchResult = {
  generation: 42,
  glyph_count: 1,
  layer_count: 1,
  decomposed_layers: 0,
  approximated_layers: 0,
  approximated_sources: [],
  examples: [
    {
      glyph: 'a',
      master: 'Regular',
      before: 'M0 0L100 0L100 100Z',
      after: 'M20 0L120 0L120 100Z',
      before_width: 600,
      after_width: 600,
      bounds: [0, 0, 120, 100],
    },
  ],
  change: null,
}

function activeGlyph(): ActiveGlyph {
  const layer = makeLayer()
  const glyph = makeGlyph({ layers: [layer] })
  return {
    doc: 1,
    gid: 7,
    glyph,
    scene: buildScene({
      detail: makeGlyphDetail(glyph),
      cell: null,
      master: null,
      masterId: 'm1',
      unitsPerEm: 1000,
    }),
    cell: null,
    outline: makeLayerOutline(layer),
    master: 'm1',
    masterGuides: [],
    reload: vi.fn(),
  }
}

function renderFilter() {
  const active = activeGlyph()
  useEditorStore.getState().setActiveGlyph(active)
  usePluginStore.getState().install(pluginId)
  usePluginStore.getState().enable(pluginId)
  return { ...render(<PluginManager />), active, user: userEvent.setup() }
}

async function preview(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Preview filter' }))
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Apply filter' })).toBeEnabled(),
  )
}

beforeEach(() => {
  useFilterPreview.getState().cancel()
  usePluginStore.setState({ installed: {} })
  useEditorStore.getState().setActiveGlyph(null)
  localStorage.removeItem(PLUGIN_STORAGE_KEY)
})

afterEach(() => {
  cleanup()
  useFilterPreview.getState().cancel()
  localStorage.removeItem(PLUGIN_STORAGE_KEY)
})

it('requires installation and enabling before exposing filter controls', async () => {
  const transport = createMockTransport()
  setTransport(transport)
  const user = userEvent.setup()
  render(<PluginManager />)

  expect(screen.queryByRole('spinbutton')).not.toBeInTheDocument()
  await user.click(
    screen.getByRole('button', { name: 'Install Translate filter' }),
  )
  expect(screen.getByText('Disabled')).toBeVisible()
  expect(screen.queryByRole('spinbutton')).not.toBeInTheDocument()

  await user.click(
    screen.getByRole('button', { name: 'Enable Translate filter' }),
  )
  expect(
    screen.getByRole('spinbutton', { name: 'Horizontal shift' }),
  ).toHaveValue(20)
  expect(screen.getByRole('button', { name: 'Preview filter' })).toBeDisabled()
  expect(screen.getByRole('status')).toHaveTextContent(
    'Open a glyph’s master layer',
  )

  await user.click(
    screen.getByRole('button', { name: 'Disable Translate filter' }),
  )
  expect(screen.queryByRole('spinbutton')).not.toBeInTheDocument()
  await user.click(
    screen.getByRole('button', { name: 'Remove Translate filter' }),
  )
  expect(
    screen.getByRole('button', { name: 'Install Translate filter' }),
  ).toBeEnabled()
  expect(transport.calls).toEqual([])
})

it('previews the active master without mutation and Cancel prevents Apply', async () => {
  const transport = createMockTransport({ 'style.batch': () => result })
  setTransport(transport)
  const { user, active } = renderFilter()
  await preview(user)

  expect(screen.getByText(/1 master layer at revision 42/)).toBeVisible()
  expect(
    screen.getByRole('img', { name: 'Original and proposed filter outlines' }),
  ).toBeVisible()
  expect(transport.calls).toEqual([
    {
      method: 'style.batch',
      params: {
        doc: 1,
        glyphs: [7],
        master: 'm1',
        options: {
          scale: [1, 1],
          translate: [20, 0],
          oblique: 0,
          weight: [0, 0],
        },
        scale_advance: false,
        center: false,
        backup: false,
        label: 'Translate filter',
        dry_run: true,
        expected_generation: null,
      },
    },
  ])

  await user.click(screen.getByRole('button', { name: 'Cancel preview' }))
  expect(screen.queryByRole('img')).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Apply filter' })).toBeDisabled()
  await user.click(screen.getByRole('button', { name: 'Apply filter' }))
  expect(transport.calls).toHaveLength(1)
  expect(active.reload).not.toHaveBeenCalled()
})

it('requires a new preview after changing an offset', async () => {
  const transport = createMockTransport({ 'style.batch': () => result })
  setTransport(transport)
  const { user } = renderFilter()
  await preview(user)
  await user.clear(screen.getByRole('spinbutton', { name: 'Horizontal shift' }))
  await user.type(
    screen.getByRole('spinbutton', { name: 'Horizontal shift' }),
    '35',
  )

  expect(screen.queryByRole('img')).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Apply filter' })).toBeDisabled()
  expect(transport.calls).toHaveLength(1)
  await preview(user)
  expect(transport.calls[1]?.params).toMatchObject({
    dry_run: true,
    options: { translate: [35, 0] },
  })
})

it('applies the reviewed revision once and reloads the active glyph', async () => {
  const transport = createMockTransport({ 'style.batch': () => result })
  setTransport(transport)
  const { user, active } = renderFilter()
  await preview(user)
  await user.click(screen.getByRole('button', { name: 'Apply filter' }))

  await waitFor(() => expect(active.reload).toHaveBeenCalledOnce())
  expect(transport.calls).toHaveLength(2)
  expect(transport.calls[1]).toEqual({
    method: 'style.batch',
    params: {
      ...(transport.calls[0]?.params as object),
      dry_run: false,
      expected_generation: 42,
    },
  })
  expect(screen.queryByRole('img')).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Apply filter' })).toBeDisabled()
})

it('explains stale-preview rejection without reloading or allowing another Apply', async () => {
  const transport = createMockTransport({
    'style.batch': ({ dry_run }) => {
      if (!dry_run) throw new Error('Style preview is stale')
      return result
    },
  })
  setTransport(transport)
  const { user, active } = renderFilter()
  await preview(user)
  await user.click(screen.getByRole('button', { name: 'Apply filter' }))

  expect(await screen.findByRole('alert')).toHaveTextContent(
    'The document changed after this preview. Preview the filter again.',
  )
  expect(active.reload).not.toHaveBeenCalled()
  expect(screen.queryByRole('img')).not.toBeInTheDocument()
  expect(screen.getByRole('button', { name: 'Apply filter' })).toBeDisabled()
  expect(transport.calls).toHaveLength(2)
})

it.each(['closing the manager', 'switching glyphs'])(
  'releases the preview when %s',
  async (action) => {
    const transport = createMockTransport({ 'style.batch': () => result })
    setTransport(transport)
    const { user, unmount, active } = renderFilter()
    await preview(user)

    if (action === 'closing the manager') unmount()
    else {
      act(() => useEditorStore.getState().setActiveGlyph({ ...active, gid: 8 }))
      expect(
        screen.getByRole('button', { name: 'Apply filter' }),
      ).toBeDisabled()
    }
    expect(useFilterPreview.getState().preview).toBeNull()
    expect(transport.calls).toHaveLength(1)
    expect(active.reload).not.toHaveBeenCalled()
  },
)
