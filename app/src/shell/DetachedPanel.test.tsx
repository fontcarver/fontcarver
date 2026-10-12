import { render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, expect, it } from 'vitest'

import { setTransport } from '../api'
import { initialDocumentState, useDocumentStore } from '../stores/documentStore'
import { initialEditorState, useEditorStore } from '../stores/editorStore'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, tabId, useUiStore, type Tab } from '../stores/uiStore'
import {
  makeCell,
  makeFontInfoView,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
} from '../test/fontFixtures'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { DetachedPanel } from './DetachedPanel'

const summary = makeSummary({
  masters: [{ id: 'master-1', name: 'Regular' }],
})

/** A glyph tab as it arrives in a torn-off window. */
const glyphTab = (gid: number): Tab => ({
  id: tabId('glyph', summary.doc, gid),
  kind: 'glyph',
  doc: summary.doc,
  gid,
})

function transportFor(name: string) {
  const glyph = makeGlyph({ name, layers: [makeLayer()] })
  const layer = glyph.layers[0]
  return createMockTransport({
    'glyphs.detail': () => makeGlyphDetail(glyph),
    'glyphs.cell': () => makeCell({ name }),
    'glyphs.list': () => ({ total: 0, items: [] }),
    'doc.font_info': () => makeFontInfoView(),
    'glyphs.outline': () => makeLayerOutline(layer),
  })
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useEditorStore.setState(initialEditorState())
  useDocumentStore.setState(initialDocumentState())
  useSessionStore.setState({ ...initialSessionState(), docs: [summary] })
  useDocumentStore.getState().ensure(summary.doc)
})

afterEach(() => setTransport(undefined))

it('publishes the glyph for an Inspector torn off an Edit tab', async () => {
  setTransport(transportFor('A'))
  const tab = glyphTab(1)
  useUiStore.setState({ tabs: [tab], activeTab: tab.id })

  render(
    <DetachedPanel
      target={{ kind: 'panel', doc: summary.doc, panel: 'inspector' }}
      summary={summary}
      tab={tab}
    />,
  )

  // The panel window has no canvas, so without a published glyph the editing
  // panels would wait for one for ever.
  await waitFor(() =>
    expect(useEditorStore.getState().activeGlyph?.gid).toBe(1),
  )
  expect(useEditorStore.getState().activeGlyph?.outline).not.toBeNull()
  expect(screen.queryByText('Loading glyph…')).not.toBeInTheDocument()
})

it('publishes the glyph for a sidebar torn off an Edit tab and follows the glyph', async () => {
  setTransport(transportFor('A'))
  const tab = glyphTab(1)
  useUiStore.setState({ tabs: [tab], activeTab: tab.id })

  const view = render(
    <DetachedPanel
      target={{ kind: 'panel', doc: summary.doc, panel: 'sidebar' }}
      summary={summary}
      tab={tab}
    />,
  )
  await waitFor(() =>
    expect(useEditorStore.getState().activeGlyph?.gid).toBe(1),
  )

  // The owning window moves the tab to another glyph; the window follows.
  setTransport(transportFor('B'))
  const moved = glyphTab(2)
  view.rerender(
    <DetachedPanel
      target={{ kind: 'panel', doc: summary.doc, panel: 'sidebar' }}
      summary={summary}
      tab={moved}
    />,
  )
  await waitFor(() =>
    expect(useEditorStore.getState().activeGlyph?.gid).toBe(2),
  )
})

it('publishes nothing for the Overview or the preview', async () => {
  setTransport(transportFor('A'))
  const overview: Tab = {
    id: tabId('font', summary.doc),
    kind: 'font',
    doc: summary.doc,
  }
  useUiStore.setState({ tabs: [overview], activeTab: overview.id })

  render(
    <DetachedPanel
      target={{ kind: 'panel', doc: summary.doc, panel: 'inspector' }}
      summary={summary}
      tab={overview}
    />,
  )

  await Promise.resolve()
  expect(useEditorStore.getState().activeGlyph).toBeNull()
})
