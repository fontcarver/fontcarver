import { fireEvent, render, screen } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport, type LayoutInspection } from '../../api'
import { createMockTransport } from '../../test/mockTransport'
import { LayoutConversion } from './LayoutConversion'

afterEach(() => setTransport(undefined))

it('keeps successful proof internals in collapsed accessible details', async () => {
  const inspection: LayoutInspection = {
    doc: 1,
    generation: 3,
    compile_revision: 2,
    draft_revision: 0,
    authority: 'ImportedTables',
    fingerprint: 'abc',
    tables: [],
    blockers: [],
    blockers_truncated: false,
    can_convert: true,
    original_layout_available: false,
    total: 0,
    items: [],
    next_offset: null,
  }
  setTransport(
    createMockTransport({
      'layout.prepare_conversion': () => ({
        doc: 1,
        master: 'm1',
        generation: 3,
        compile_revision: 2,
        draft_revision: 0,
        fingerprint: 'abc',
        token: 'proof',
        expires_in_seconds: 60,
        source: null,
        kerning_lifted: false,
        anchors_lifted: false,
        messages: ['Optional lift proof details'],
        blockers: [],
      }),
    }),
  )
  render(
    <LayoutConversion
      inspection={inspection}
      master="m1"
      onChanged={vi.fn()}
    />,
  )
  fireEvent.click(screen.getByRole('button', { name: 'Check conversion' }))
  const message = await screen.findByText('Optional lift proof details')
  const details = message.closest('details')
  expect(details).not.toHaveAttribute('open')
  expect(message).not.toBeVisible()
  expect(
    screen.getByRole('button', { name: 'Use editable source' }),
  ).toBeVisible()
  fireEvent.click(screen.getByText('Details'))
  expect(details).toHaveAttribute('open')
  expect(message).toBeVisible()
})
