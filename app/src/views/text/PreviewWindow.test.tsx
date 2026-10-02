import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, expect, it } from 'vitest'

import { setTransport } from '../../api'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { initialUiState, useUiStore } from '../../stores/uiStore'
import { makeCell } from '../../test/fontFixtures'
import { createMockTransport, makeSummary } from '../../test/mockTransport'
import { PreviewWindow } from './PreviewWindow'
import { initialTextState } from './textState'

const summary = makeSummary()

function transport() {
  return createMockTransport({
    'glyphs.cell': ({ gid }) =>
      makeCell({
        gid,
        name: 'A',
        width: 600,
        path: 'M40 0L560 0L560 700L40 700Z',
      }),
    'glyphs.search': () => [
      makeCell({
        gid: 1,
        name: 'A',
        width: 600,
        path: 'M40 0L560 0L560 700L40 700Z',
      }),
    ],
    'text.shape': ({ text }) => ({
      glyphs: Array.from(text).map((_, cluster) => ({
        gid: 1,
        cluster,
        x_advance: 600,
        y_advance: 0,
        x_offset: 0,
        y_offset: 0,
      })),
      names: Array.from(text).map(() => 'A'),
      units_per_em: 1000,
      size: null,
      stale: false,
      warnings: [],
    }),
  })
}

function tabWith(state: Partial<ReturnType<typeof initialTextState>>) {
  const id = useUiStore.getState().openTab({
    kind: 'glyph',
    doc: 1,
    gid: 1,
    textState: { ...initialTextState('m1', 'AA'), ...state },
  })
  const tab = useUiStore.getState().tabs.find((entry) => entry.id === id)!
  return tab
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useDocumentStore.setState(initialDocumentState())
})
afterEach(() => setTransport(undefined))

it('shows the waterfall, flip and blur the docked strip stored on the tab', async () => {
  setTransport(transport())
  const tab = tabWith({
    previewSizes: [10, 20],
    previewFlipHorizontal: true,
    previewBlur: 2,
    previewMode: 'single',
  })
  render(<PreviewWindow summary={summary} tab={tab} />)
  const strip = await waitFor(() => {
    const node = document.querySelector('[data-preview-theme]')
    if (!node) throw new Error('strip is not mounted')
    return node
  })
  expect(strip).toHaveAttribute('data-preview-sizes', '10, 20')
  expect(strip).toHaveAttribute('data-preview-mode', 'single')
  expect(
    screen.getByRole('button', { name: 'Flip preview horizontally' }),
  ).toHaveAttribute('aria-pressed', 'true')
  fireEvent.click(screen.getByRole('button', { name: 'Preview settings' }))
  expect(screen.getByRole('spinbutton', { name: 'Blur (px)' })).toHaveValue(2)
  expect(screen.getByRole('img', { name: '10 px preview' })).toBeVisible()
  expect(screen.getByRole('img', { name: '20 px preview' })).toBeVisible()
})

it('writes its own changes back to the tab, so the docked strip follows', async () => {
  setTransport(transport())
  const tab = tabWith({})
  render(<PreviewWindow summary={summary} tab={tab} />)
  await waitFor(() =>
    expect(document.querySelector('[data-preview-theme]')).not.toBeNull(),
  )
  fireEvent.click(screen.getByRole('button', { name: 'Preview settings' }))
  fireEvent.change(
    screen.getByRole('textbox', { name: 'Waterfall sizes (px)' }),
    { target: { value: '16-48/16' } },
  )
  fireEvent.click(screen.getByRole('button', { name: 'Apply sizes' }))
  await waitFor(() =>
    expect(
      useUiStore.getState().tabs.find((entry) => entry.id === tab.id)?.textState
        ?.previewSizes,
    ).toEqual([16, 32, 48]),
  )
  fireEvent.click(
    screen.getByRole('button', { name: 'Flip preview vertically' }),
  )
  await waitFor(() =>
    expect(
      useUiStore.getState().tabs.find((entry) => entry.id === tab.id)?.textState
        ?.previewFlipVertical,
    ).toBe(true),
  )
})
