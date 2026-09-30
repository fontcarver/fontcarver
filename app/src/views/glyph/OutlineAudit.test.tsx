import { createRef } from 'react'
import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type ApplyParams } from '../../api'
import { Viewport } from '../../canvas/Viewport'
import type { EditorSource } from '../../editor/editorController'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import {
  makeContour,
  makeLayer,
  makeLayerOutline,
  makePoint,
} from '../../test/fontFixtures'
import { createMockTransport, noChange } from '../../test/mockTransport'

import { OutlineAudit, type OutlineAuditHandle } from './OutlineAudit'

const outline = makeLayerOutline(
  makeLayer({
    contours: [
      makeContour([
        makePoint(10.2, 20.6),
        makePoint(100.7, 20.2),
        makePoint(100, 200),
      ]),
    ],
  }),
)
const source: EditorSource = {
  doc: 1,
  gid: 42,
  glyph: 'a',
  outline,
  master: 'm1',
  guides: [],
  metrics: {
    upem: 1000,
    width: 600,
    lsb: 10.2,
    rsb: 499.3,
    ascender: 800,
    descender: -200,
    xHeight: 500,
    capHeight: 700,
    italicAngle: 0,
    verticalOrigin: null,
    verticalWidth: null,
  },
  curves: 'cubic',
  componentPoints: [],
}

beforeEach(() => useEditorStore.setState(initialEditorState()))
afterEach(() => setTransport(undefined))

async function show(props: Partial<Parameters<typeof OutlineAudit>[0]> = {}) {
  const user = userEvent.setup()
  const viewport = new Viewport()
  viewport.offset = { x: 100, y: 300 }
  const onReload = vi.fn()
  const view = render(
    <OutlineAudit
      source={source}
      viewport={viewport}
      onReload={onReload}
      {...props}
    />,
  )
  await user.click(screen.getByText(/Outline audit ·/))
  return { user, onReload, viewport, ...view }
}

describe('outline audit controls', () => {
  it('shows hover details and updates marker position after pan without changing the source', async () => {
    const handleRef = createRef<OutlineAuditHandle>()
    const { viewport } = await show({ handleRef })
    const marker = screen.getByRole('button', {
      name: 'Fractional coordinates at 10.2, 20.6',
    })
    expect(marker).toHaveAttribute(
      'title',
      expect.stringContaining('nearest font unit'),
    )
    expect(marker.style.left).toBe('110.2px')
    viewport.pan(40, 10)
    act(() => handleRef.current?.refresh())
    expect(marker.style.left).toBe('150.2px')
  })

  it('hides only disabled check markers and disables Fix All when nothing fixable is shown', async () => {
    const { user } = await show()
    await user.click(screen.getByLabelText('Fractional coordinates'))
    expect(
      screen.queryByRole('button', { name: /Fractional coordinates at/ }),
    ).toBeNull()
    expect(screen.getByRole('button', { name: 'Fix All' })).toBeDisabled()
    expect(screen.getByText('No issues in the enabled checks.')).toBeVisible()
  })

  it('applies all displayed fixes as one command after refreshing the outline', async () => {
    const transport = createMockTransport({
      'glyphs.outline': () => outline,
      'doc.apply': () => noChange([42]),
    })
    setTransport(transport)
    const { user, onReload } = await show()
    await user.click(screen.getByRole('button', { name: 'Fix All' }))
    await waitFor(() => expect(onReload).toHaveBeenCalledOnce())
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'glyphs.outline',
      'doc.apply',
    ])
    const applied = transport.calls[1]?.params as ApplyParams
    expect('Batch' in applied.command && applied.command.Batch).toHaveLength(2)
  })

  it('refuses a stale source before applying and reloads it', async () => {
    const transport = createMockTransport({
      'glyphs.outline': () => ({ ...outline, contours: [] }),
    })
    setTransport(transport)
    const { user, onReload } = await show()
    await user.click(screen.getAllByRole('button', { name: 'Fix' })[0]!)
    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent(
        'The outline changed',
      ),
    )
    expect(transport.calls.some((entry) => entry.method === 'doc.apply')).toBe(
      false,
    )
    expect(onReload).toHaveBeenCalledOnce()
  })

  it('reports a failed mutation and permits retry after reconciliation', async () => {
    setTransport(
      createMockTransport({
        'glyphs.outline': () => outline,
        'doc.apply': () => {
          throw new Error('Refused audit repair')
        },
      }),
    )
    const { user, onReload } = await show()
    await user.click(screen.getByRole('button', { name: 'Fix All' }))
    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent(
        'Refused audit repair',
      ),
    )
    expect(onReload).toHaveBeenCalledOnce()
    expect(screen.getByRole('button', { name: 'Fix All' })).toBeEnabled()
  })

  it('disables repairs during a live editing gesture', async () => {
    useEditorStore.setState({ editing: true })
    await show()
    expect(screen.getByRole('button', { name: 'Fix All' })).toBeDisabled()
    expect(
      screen.queryByRole('button', { name: /Fractional coordinates at/ }),
    ).toBeNull()
  })

  it('keeps locked paths and glyphs read-only', async () => {
    const locked = { ...source, glyphLocked: true }
    const { rerender, viewport, onReload } = await show({ source: locked })
    expect(screen.getByRole('button', { name: 'Fix All' })).toBeDisabled()
    rerender(
      <OutlineAudit
        source={{
          ...source,
          editorMetadata: {
            'com.fontcarver.editor': { locked_paths: ['locked-path'] },
          },
          outline: {
            ...outline,
            contours: outline.contours.map((contour) => ({
              ...contour,
              id: 'locked-path',
            })),
          },
        }}
        viewport={viewport}
        onReload={onReload}
      />,
    )
    expect(screen.getByRole('button', { name: 'Fix All' })).toBeDisabled()
  })
})
