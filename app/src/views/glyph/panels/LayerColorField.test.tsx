import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, expect, it, vi } from 'vitest'

import { setTransport } from '../../../api'
import { i18n } from '../../../i18n'
import { makeLayer } from '../../../test/fontFixtures'
import { createMockTransport, noChange } from '../../../test/mockTransport'
import { LayerColorField, layerColor } from './LayerColorField'

beforeEach(async () => {
  await i18n.changeLanguage('en')
})

it('edits and clears the selected layer label independently through undoable commands', async () => {
  const mock = createMockTransport({ 'doc.apply': () => noChange([1]) })
  setTransport(mock)
  const user = userEvent.setup()
  const reload = vi.fn()
  const layer = makeLayer({
    user_data: { 'com.fontcarver.colorLabel': 3, vendor: 42 },
  })
  const target = { doc: 1, glyph: 'A', layer: layer.id }
  const { rerender } = render(
    <LayerColorField target={target} layer={layer} reload={reload} />,
  )
  const input = screen.getByRole('combobox', { name: 'Layer color' })
  expect(input).toHaveValue('3')
  await user.selectOptions(input, '11')
  await waitFor(() => expect(reload).toHaveBeenCalledOnce())
  expect(
    mock.calls.find((call) => call.method === 'doc.apply')?.params,
  ).toMatchObject({
    command: { SetLayerColor: { glyph: 'A', layer: layer.id, color: 11 } },
  })
  rerender(
    <LayerColorField
      target={target}
      layer={{ ...layer, user_data: { 'com.fontcarver.colorLabel': 11 } }}
      reload={reload}
    />,
  )
  await user.selectOptions(input, '')
  await waitFor(() => expect(reload).toHaveBeenCalledTimes(2))
  expect(
    mock.calls.filter((call) => call.method === 'doc.apply').at(-1)?.params,
  ).toMatchObject({
    command: { SetLayerColor: { glyph: 'A', layer: layer.id, color: null } },
  })
})

it('treats malformed imported label data as unset without changing it', () => {
  for (const data of [
    null,
    [],
    3,
    { 'com.fontcarver.colorLabel': '1' },
    { 'com.fontcarver.colorLabel': 12 },
  ]) {
    const layer = makeLayer({ user_data: data })
    expect(layerColor(layer)).toBeNull()
    expect(layer.user_data).toEqual(data)
  }
})
