import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport, type ApplyParams } from '../api'
import { initialSessionState, useSessionStore } from '../stores/sessionStore'
import { initialUiState, useUiStore } from '../stores/uiStore'
import { makeFontInfoView } from '../test/fontFixtures'
import { createMockTransport, makeSummary } from '../test/mockTransport'

import { NewFontDialog } from './NewFontDialog'

function setup() {
  const created = makeSummary({
    doc: 4,
    path: null,
    family_name: 'New Font',
    style_name: 'Regular',
    units_per_em: 1000,
    glyph_count: 0,
  })
  const transport = createMockTransport({
    'session.new': () => created,
    'doc.font_info': () => makeFontInfoView(),
    'doc.apply': () => ({
      glyphs: [],
      font_info: true,
      masters: true,
      glyph_order: false,
      metrics: false,
      layers: false,
      guides: false,
    }),
    'glyphs.add': () => ({ added: [], skipped: [] }),
    'doc.summary': () => created,
  })
  setTransport(transport)
  useUiStore.getState().setDialog({ kind: 'newFont' })
  render(<NewFontDialog />)
  return transport
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useSessionStore.setState(initialSessionState())
})

afterEach(() => {
  setTransport(undefined)
})

describe('NewFontDialog', () => {
  it('creates the font, its metrics batch and its starter glyphs', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.clear(screen.getByLabelText('Family name'))
    await user.type(screen.getByLabelText('Family name'), 'Workshop Sans')
    await user.clear(screen.getByLabelText('Style name'))
    await user.type(screen.getByLabelText('Style name'), 'Bold')
    await user.click(screen.getByRole('radio', { name: '2048' }))
    await user.click(screen.getByRole('button', { name: 'Create' }))

    await waitFor(() => expect(useUiStore.getState().dialog).toBeNull())
    expect(transport.calls.map((entry) => entry.method)).toEqual([
      'session.new',
      'doc.font_info',
      'doc.apply',
      'glyphs.add',
      'doc.summary',
    ])
    expect(transport.calls[0]?.params).toEqual({
      family: 'Workshop Sans',
      units_per_em: 2048,
    })

    const apply = transport.calls[2]?.params as ApplyParams
    expect(apply.doc).toBe(4)
    expect(apply.command).toEqual({
      Batch: [
        {
          SetFontInfo: {
            info: expect.objectContaining({
              family_name: 'Workshop Sans',
              style_name: 'Bold',
              units_per_em: 2048,
            }),
          },
        },
        {
          SetMasterMetrics: {
            master: 'm1',
            metrics: expect.objectContaining({
              ascender: 1638,
              descender: -410,
              x_height: 1024,
              cap_height: 1434,
            }),
          },
        },
      ],
    })
    expect(transport.calls[3]?.params).toEqual({
      doc: 4,
      recipe: expect.stringContaining('A-Z a-z 0-9'),
      master: null,
      dry_run: false,
    })
  })

  it('pins the CJK preset to a 1000-unit em with 880 over −120', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.click(screen.getByRole('radio', { name: '2048' }))
    await user.click(screen.getByRole('radio', { name: 'CJK' }))
    await user.click(screen.getByRole('button', { name: 'Create' }))

    await waitFor(() => expect(useUiStore.getState().dialog).toBeNull())
    expect(transport.calls[0]?.params).toMatchObject({ units_per_em: 1000 })
    const apply = transport.calls[2]?.params as ApplyParams
    expect(apply.command).toMatchObject({
      Batch: [
        {},
        {
          SetMasterMetrics: {
            metrics: expect.objectContaining({
              ascender: 880,
              descender: -120,
            }),
          },
        },
      ],
    })
  })

  it('creates no starter glyphs when the set is None', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.click(screen.getByRole('radio', { name: 'None' }))
    await user.click(screen.getByRole('button', { name: 'Create' }))

    await waitFor(() => expect(useUiStore.getState().dialog).toBeNull())
    expect(transport.calls.map((entry) => entry.method)).not.toContain(
      'glyphs.add',
    )
  })

  it('refuses to create a font without a family name', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.clear(screen.getByLabelText('Family name'))

    expect(screen.getByRole('button', { name: 'Create' })).toBeDisabled()
    expect(transport.calls).toHaveLength(0)
  })

  it('closes on Escape without touching the kernel', async () => {
    const user = userEvent.setup()
    const transport = setup()

    await user.keyboard('{Escape}')

    expect(useUiStore.getState().dialog).toBeNull()
    expect(transport.calls).toHaveLength(0)
  })
})
