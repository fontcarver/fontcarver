import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type SmartFilterNode } from '../../api'
import { createMockTransport } from '../../test/mockTransport'

import { SmartFilterEditor } from './SmartFilterEditor'

afterEach(() => setTransport(undefined))

const parsedRoot: SmartFilterNode = {
  kind: 'all',
  children: [
    { kind: 'predicate', field: 'script', operator: 'eq', value: 'han' },
    {
      kind: 'none',
      children: [
        { kind: 'predicate', field: 'export', operator: 'eq', value: 'true' },
      ],
    },
  ],
}

describe('SmartFilterEditor', () => {
  it('round-trips a parsed custom expression through nested visual rules and canonical save', async () => {
    const user = userEvent.setup()
    const transport = createMockTransport({
      'glyph.smart_filters.parse': ({ expression }) => ({
        expression,
        root: parsedRoot,
      }),
    })
    setTransport(transport)
    const onSave = vi.fn().mockResolvedValue(undefined)
    render(
      <SmartFilterEditor
        initialFilter={{
          name: 'Han review',
          expression: 'script:han and not export',
        }}
        onSave={onSave}
        onCancel={vi.fn()}
      />,
    )

    await user.click(screen.getByRole('button', { name: 'Visual rules' }))
    await waitFor(() =>
      expect(screen.getAllByLabelText('Match rules')).toHaveLength(2),
    )
    expect(screen.getAllByLabelText('Match rules')[1]).toHaveValue('none')
    await user.selectOptions(screen.getAllByLabelText('Match rules')[1], 'any')
    await user.click(screen.getByRole('button', { name: 'Custom expression' }))
    expect(screen.getByLabelText('Filter expression')).toHaveValue(
      '(script:"han" and (export:"true"))',
    )
    await user.click(screen.getByRole('button', { name: 'Save filter' }))
    await waitFor(() =>
      expect(onSave).toHaveBeenCalledWith({
        name: 'Han review',
        expression: '(script:"han" and (export:"true"))',
      }),
    )
    expect(
      transport.calls.filter(
        ({ method }) => method === 'glyph.smart_filters.parse',
      ),
    ).toHaveLength(2)
  })

  it('keeps malformed expressions editable and does not save them', async () => {
    const user = userEvent.setup()
    setTransport(
      createMockTransport({
        'glyph.smart_filters.parse': () => {
          throw new Error('Unclosed group')
        },
      }),
    )
    const onSave = vi.fn()
    render(
      <SmartFilterEditor
        initialFilter={{ name: 'Broken', expression: '(name:A' }}
        onSave={onSave}
        onCancel={vi.fn()}
      />,
    )
    await user.click(screen.getByRole('button', { name: 'Visual rules' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Unclosed group')
    expect(screen.getByLabelText('Filter expression')).toHaveValue('(name:A')
    await user.click(screen.getByRole('button', { name: 'Save filter' }))
    expect(onSave).not.toHaveBeenCalled()
  })

  it('adds and removes nested groups and preserves edits after a save failure', async () => {
    const user = userEvent.setup()
    setTransport(
      createMockTransport({
        'glyph.smart_filters.parse': ({ expression }) => ({
          expression,
          root: parsedRoot,
        }),
      }),
    )
    render(
      <SmartFilterEditor
        onSave={vi.fn().mockRejectedValue(new Error('Project is busy'))}
        onCancel={vi.fn()}
      />,
    )
    fireEvent.change(screen.getByLabelText('Filter name'), {
      target: { value: 'Review' },
    })
    await user.click(screen.getByRole('button', { name: 'Add group' }))
    expect(screen.getAllByLabelText('Match rules')).toHaveLength(2)
    await user.click(screen.getByRole('button', { name: 'Remove group' }))
    expect(screen.getAllByLabelText('Match rules')).toHaveLength(1)
    await user.click(screen.getByRole('button', { name: 'Save filter' }))
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Project is busy',
    )
    expect(screen.getByLabelText('Filter name')).toHaveValue('Review')
    expect(screen.getByRole('button', { name: 'Save filter' })).toBeEnabled()
  })
})
