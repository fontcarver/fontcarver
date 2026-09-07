import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport } from '../../api'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import { initialSessionState, useSessionStore } from '../../stores/sessionStore'
import { initialUiState, tabId, useUiStore } from '../../stores/uiStore'
import {
  makeCell,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
} from '../../test/fontFixtures'
import { glyphPage, stubGridViewport } from '../../test/glyphFixtures'
import { createMockTransport, makeSummary } from '../../test/mockTransport'
import { resetOrders } from '../font/glyphOrder'

import { EditSidebar } from './EditSidebar'
import { buildScene } from './glyphScene'

const summary = makeSummary()

const background = makeLayer({
  id: 'bg-1',
  kind: { Background: 'm1' },
  name: 'Background',
})

function publishGlyph() {
  const glyph = makeGlyph({ layers: [makeLayer(), background] })
  useEditorStore.setState({
    ...initialEditorState(),
    activeGlyph: {
      doc: 1,
      gid: 3,
      glyph,
      scene: buildScene({
        detail: makeGlyphDetail(glyph),
        cell: makeCell({ gid: 3 }),
        master: makeMaster(),
        masterId: 'm1',
        unitsPerEm: 1000,
      }),
      cell: makeCell({ gid: 3 }),
      outline: makeLayerOutline(makeLayer()),
      master: 'm1',
      masterGuides: [],
      reload: () => undefined,
    },
  })
}

let restoreViewport: (() => void) | undefined

beforeEach(() => {
  localStorage.clear()
  resetOrders()
  restoreViewport = stubGridViewport()
  useUiStore.setState(initialUiState())
  useSessionStore.setState({ ...initialSessionState(), docs: [summary] })
  useDocumentStore.setState(initialDocumentState())
  useEditorStore.setState(initialEditorState())
  setTransport(
    createMockTransport({
      'glyphs.list': (params) => glyphPage(params.offset, params.limit, 12),
      'glyphs.index_of': (params) => params.gid,
      'doc.font_info': () => {
        throw new Error('not needed')
      },
    }),
  )
})

afterEach(() => {
  restoreViewport?.()
  setTransport(undefined)
})

describe('EditSidebar', () => {
  it('lists the layers of the glyph on screen', () => {
    publishGlyph()
    render(<EditSidebar doc={1} />)

    expect(screen.getByRole('button', { name: 'Regular' })).toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Background' }),
    ).toBeInTheDocument()
  })

  it('switches the edited layer from the sidebar', async () => {
    const user = userEvent.setup()
    publishGlyph()
    render(<EditSidebar doc={1} />)

    await user.click(screen.getByRole('button', { name: 'Background' }))

    expect(useEditorStore.getState().editedLayer).toBe('bg-1')
  })

  it('replaces the tab glyph when a cell of the compact list is clicked', async () => {
    const user = userEvent.setup()
    publishGlyph()
    useUiStore.getState().openTab({ kind: 'glyph', doc: 1, gid: 3 })
    render(<EditSidebar doc={1} />)

    const list = screen.getByRole('listbox', { name: 'Glyph list' })
    const cell = await waitFor(() => {
      const found = list.querySelector('[data-gid="5"]')
      if (!found) throw new Error('cell not loaded yet')
      return found as HTMLElement
    })
    await user.click(cell)

    const tabs = useUiStore.getState().tabs
    expect(tabs.map((tab) => tab.id)).toEqual([tabId('glyph', 1, 5)])
  })
})
