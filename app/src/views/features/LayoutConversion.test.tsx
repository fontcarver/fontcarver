import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'

import {
  setTransport,
  type LayoutConversionPreview,
  type LayoutInspection,
} from '../../api'
import { i18n } from '../../i18n'
import { registerDocumentEditParticipant } from '../../editor/documentEditParticipants'
import { createMockTransport, noChange } from '../../test/mockTransport'

import english from '../../i18n/en.json'
import { LayoutConversion } from './LayoutConversion'

beforeAll(() =>
  i18n.addResourceBundle('en', 'translation', english, true, true),
)
afterEach(() => setTransport(undefined))

function inspection(
  overrides: Partial<LayoutInspection> = {},
): LayoutInspection {
  return {
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
    ...overrides,
  }
}
function preview(
  overrides: Partial<LayoutConversionPreview> = {},
): LayoutConversionPreview {
  return {
    doc: 1,
    master: 'm1',
    generation: 3,
    compile_revision: 2,
    draft_revision: 0,
    fingerprint: 'abc',
    token: 'proof',
    expires_in_seconds: 60,
    source: null,
    kerning_lifted: true,
    anchors_lifted: false,
    messages: [],
    blockers: [],
    ...overrides,
  }
}

describe('LayoutConversion', () => {
  it('prepares a proven candidate and applies only the returned token', async () => {
    const transport = createMockTransport({
      'layout.prepare_conversion': () => preview(),
      'layout.apply_conversion': () => noChange(),
    })
    setTransport(transport)
    const changed = vi.fn()
    render(
      <LayoutConversion
        inspection={inspection()}
        master="m1"
        onChanged={changed}
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Check conversion' }))
    fireEvent.click(
      await screen.findByRole('button', { name: 'Use editable source' }),
    )
    await waitFor(() => expect(changed).toHaveBeenCalledOnce())
    expect(transport.calls).toContainEqual({
      method: 'layout.apply_conversion',
      params: { doc: 1, token: 'proof' },
    })
  })

  it('never applies a blocked candidate and explains the backend reason', async () => {
    const transport = createMockTransport({
      'layout.prepare_conversion': () =>
        preview({ token: null, blockers: ['Unsupported FeatureVariations'] }),
    })
    setTransport(transport)
    render(
      <LayoutConversion
        inspection={inspection()}
        master="m1"
        onChanged={vi.fn()}
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Check conversion' }))
    expect(
      await screen.findByText('Unsupported FeatureVariations'),
    ).toBeVisible()
    expect(
      screen.queryByRole('button', { name: 'Use editable source' }),
    ).not.toBeInTheDocument()
  })

  it('allows source-only conversion without requesting model lifting', async () => {
    const transport = createMockTransport({
      'layout.prepare_conversion': () => preview({ kerning_lifted: false }),
    })
    setTransport(transport)
    render(
      <LayoutConversion
        inspection={inspection()}
        master="m1"
        onChanged={vi.fn()}
      />,
    )
    fireEvent.click(
      screen.getByRole('checkbox', {
        name: 'Use pair, group and anchor controls where equivalent',
      }),
    )
    fireEvent.click(screen.getByRole('button', { name: 'Check conversion' }))
    expect(
      await screen.findByText('Any kerning rules remain in feature source.'),
    ).toBeVisible()
    expect(transport.calls[0]).toEqual({
      method: 'layout.prepare_conversion',
      params: {
        doc: 1,
        master: 'm1',
        expected_generation: 3,
        expected_compile_revision: 2,
        expected_draft_revision: 0,
        prefer_model_lift: false,
      },
    })
  })

  it('disables apply when the server-held proof expires', async () => {
    setTransport(
      createMockTransport({
        'layout.prepare_conversion': () => preview({ expires_in_seconds: 0 }),
      }),
    )
    render(
      <LayoutConversion
        inspection={inspection()}
        master="m1"
        onChanged={vi.fn()}
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Check conversion' }))
    expect(
      await screen.findByText(
        'This conversion check has expired. Check conversion again.',
      ),
    ).toBeVisible()
    expect(
      screen.getByRole('button', { name: 'Use editable source' }),
    ).toBeDisabled()
  })

  it('rejects a proof that does not cover the displayed draft revision', async () => {
    setTransport(
      createMockTransport({
        'layout.prepare_conversion': () => preview({ draft_revision: 2 }),
      }),
    )
    render(
      <LayoutConversion
        inspection={inspection()}
        master="m1"
        onChanged={vi.fn()}
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Check conversion' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'The font or selected master changed.',
    )
    expect(
      screen.queryByRole('button', { name: 'Use editable source' }),
    ).not.toBeInTheDocument()
  })

  it('ignores a proof returned after a master or revision switch', async () => {
    let finish: (value: LayoutConversionPreview) => void = () => undefined
    setTransport(
      createMockTransport({
        'layout.prepare_conversion': () =>
          new Promise((resolve) => {
            finish = resolve
          }),
      }),
    )
    const view = render(
      <LayoutConversion
        inspection={inspection()}
        master="m1"
        onChanged={vi.fn()}
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Check conversion' }))
    view.rerender(
      <LayoutConversion
        inspection={inspection({ generation: 4 })}
        master="m2"
        onChanged={vi.fn()}
      />,
    )
    await act(async () => finish(preview()))
    expect(
      screen.queryByRole('button', { name: 'Use editable source' }),
    ).not.toBeInTheDocument()
    expect(
      screen.getByRole('button', { name: 'Check conversion' }),
    ).toBeEnabled()
  })

  it('keeps the original authority and shows a failed apply', async () => {
    const changed = vi.fn()
    setTransport(
      createMockTransport({
        'layout.prepare_conversion': () => preview(),
        'layout.apply_conversion': () => {
          throw new Error('proof expired')
        },
      }),
    )
    render(
      <LayoutConversion
        inspection={inspection()}
        master="m1"
        onChanged={changed}
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: 'Check conversion' }))
    fireEvent.click(
      await screen.findByRole('button', { name: 'Use editable source' }),
    )
    expect(await screen.findByRole('alert')).toHaveTextContent('proof expired')
    expect(changed).not.toHaveBeenCalled()
    expect(
      screen.queryByRole('button', { name: 'Use editable source' }),
    ).not.toBeInTheDocument()
  })

  it('requires an explicit revert action and preserves persistence failures', async () => {
    const beforeChange = vi.fn(async () => {
      throw new Error('draft save failed')
    })
    const transport = createMockTransport({
      'layout.revert_original': () => noChange(),
    })
    setTransport(transport)
    render(
      <LayoutConversion
        inspection={inspection({
          authority: 'Fea',
          original_layout_available: true,
        })}
        master="m1"
        onChanged={vi.fn()}
        beforeChange={beforeChange}
      />,
    )
    fireEvent.click(
      screen.getByRole('button', { name: 'Restore original layout…' }),
    )
    expect(transport.calls).toEqual([])
    fireEvent.click(
      screen.getByRole('button', { name: 'Restore original layout' }),
    )
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'draft save failed',
    )
    expect(transport.calls).toEqual([])
  })

  it('restores original authority using the exact displayed revisions', async () => {
    const transport = createMockTransport({
      'layout.revert_original': () => noChange(),
    })
    const changed = vi.fn()
    setTransport(transport)
    render(
      <LayoutConversion
        inspection={inspection({
          authority: 'Fea',
          original_layout_available: true,
        })}
        master="m1"
        onChanged={changed}
      />,
    )
    fireEvent.click(
      screen.getByRole('button', { name: 'Restore original layout…' }),
    )
    fireEvent.click(
      screen.getByRole('button', { name: 'Restore original layout' }),
    )
    await waitFor(() => expect(changed).toHaveBeenCalledOnce())
    expect(transport.calls).toEqual([
      {
        method: 'layout.revert_original',
        params: {
          doc: 1,
          expected_generation: 3,
          expected_compile_revision: 2,
          expected_draft_revision: 0,
        },
      },
    ])
  })

  it('suspends source edits only after persistence and releases them after a slow revert', async () => {
    let finish: () => void = () => undefined
    const suspended = vi.fn()
    const flush = vi.fn(async () => undefined)
    const dispose = registerDocumentEditParticipant(1, {
      flush,
      setSuspended: suspended,
    })
    setTransport(
      createMockTransport({
        'layout.revert_original': async () => {
          await new Promise<void>((resolve) => {
            finish = resolve
          })
          return noChange()
        },
      }),
    )
    try {
      render(
        <LayoutConversion
          inspection={inspection({
            authority: 'Fea',
            original_layout_available: true,
          })}
          master="m1"
          onChanged={vi.fn()}
        />,
      )
      fireEvent.click(
        screen.getByRole('button', { name: 'Restore original layout…' }),
      )
      fireEvent.click(
        screen.getByRole('button', { name: 'Restore original layout' }),
      )
      await waitFor(() => expect(suspended).toHaveBeenCalledWith(true))
      expect(flush.mock.invocationCallOrder[0]).toBeLessThan(
        suspended.mock.invocationCallOrder[0]!,
      )
      expect(suspended).not.toHaveBeenCalledWith(false)
      await act(async () => finish())
      expect(suspended).toHaveBeenLastCalledWith(false)
    } finally {
      dispose()
    }
  })
})
