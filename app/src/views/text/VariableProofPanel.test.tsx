import { act, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'
import { setTransport } from '../../api'
import {
  initialDocumentState,
  useDocumentStore,
} from '../../stores/documentStore'
import { createMockTransport, makeSummary } from '../../test/mockTransport'
import { initialTextState } from './textState'
import { VariableProofPanel } from './VariableProofPanel'

vi.mock('./PreviewStrip', () => ({
  PreviewStrip: ({ glyphs }: { glyphs: { path: string }[] }) => (
    <output data-testid="proof-path">{glyphs[0]?.path}</output>
  ),
}))

afterEach(() => {
  setTransport(undefined)
  vi.useRealTimers()
  useDocumentStore.setState(initialDocumentState())
})

it('opens on demand, maps named instances and requests slider positions without changing the editing master', async () => {
  vi.useFakeTimers()
  useDocumentStore.setState(initialDocumentState())
  const axis = {
    tag: 'wght',
    name: 'Weight',
    min: 100,
    default: 100,
    max: 900,
    hidden: false,
  }
  const transport = createMockTransport({
    'variation.state': () => ({
      generation: 0,
      axes: [
        {
          ...axis,
          tag: [119, 103, 104, 116],
          map: [
            [100, 0],
            [900, 1000],
          ],
        },
      ],
      masters: [],
      instances: [
        {
          name: 'Medium',
          style_name: 'Medium',
          family_name: null,
          active: true,
          is_variable_named_instance: true,
          user_data: null,
          location: { wght: 500 },
        },
      ],
      first_glyph: 'A',
    }),
    'text.variable_preview': ({ instance, location }) => ({
      generation: 0,
      units_per_em: 1000,
      warnings: [],
      location,
      glyphs: [
        {
          name: 'A',
          path: instance === 0 ? 'M0 0L200 0Z' : 'M0 0L100 0Z',
          x: 0,
          y: 0,
          x_advance: 600,
          y_advance: 0,
        },
      ],
    }),
  })
  setTransport(transport)
  const state = initialTextState('editing-master')
  render(
    <VariableProofPanel
      summary={makeSummary({ axes: [axis] })}
      state={state}
    />,
  )
  expect(transport.calls).toHaveLength(0)
  fireEvent.click(screen.getByRole('button', { name: 'Variable font proof' }))
  await act(() => vi.advanceTimersByTimeAsync(200))
  fireEvent.change(screen.getByRole('combobox'), { target: { value: '0' } })
  await act(() => vi.advanceTimersByTimeAsync(200))
  expect(screen.getByRole('slider')).toHaveValue('500')
  expect(screen.getByTestId('proof-path')).toHaveTextContent('M0 0L200 0Z')
  fireEvent.change(screen.getByRole('slider'), { target: { value: '700' } })
  await act(() => vi.advanceTimersByTimeAsync(200))
  expect(transport.calls.at(-1)?.params).toMatchObject({
    location: { wght: 700 },
    instance: null,
  })
  expect(state.master).toBe('editing-master')
  expect(
    transport.calls.every(
      ({ method }) =>
        method === 'variation.state' || method === 'text.variable_preview',
    ),
  ).toBe(true)
})
