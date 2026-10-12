import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { setTransport, type ApplyParams } from '../../../api'
import { i18n } from '../../../i18n'
import { makeGlyph, makeLayer } from '../../../test/fontFixtures'
import { createMockTransport, noChange } from '../../../test/mockTransport'
import { LayerManagementControls } from './LayerManagementControls'

afterEach(() => setTransport(undefined))

function setup(locked = false) {
  const layer = makeLayer({ name: 'Regular' })
  const glyph = makeGlyph({ layers: [layer], locked })
  const reload = vi.fn()
  const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
  setTransport(mock)
  render(
    <LayerManagementControls
      target={{ doc: 1, glyph: 'a', layer: layer.id }}
      glyph={glyph}
      current={layer}
      reload={reload}
    />,
  )
  return { mock, reload }
}

describe('named layer controls', () => {
  it.each([
    ['addCustom', 'AddLayer'],
    ['duplicate', 'AddLayer'],
    ['rename', 'RenameLayer'],
  ])('sends %s as one persistent command', async (action, variant) => {
    const { mock, reload } = setup()
    const user = userEvent.setup()
    const input = screen.getByLabelText(i18n.t('panel.layers.name'))
    await user.clear(input)
    await user.type(input, ' Alternate ')
    await user.click(
      screen.getByRole('button', { name: i18n.t(`panel.layers.${action}`) }),
    )
    await waitFor(() => expect(reload).toHaveBeenCalledOnce())
    const applies = mock.calls.filter((entry) => entry.method === 'doc.apply')
    expect(applies).toHaveLength(1)
    const command = (applies[0]?.params as ApplyParams).command
    expect(command).toHaveProperty(variant)
    if ('AddLayer' in command)
      expect(command.AddLayer.layer.name).toBe('Alternate')
    if ('RenameLayer' in command)
      expect(command.RenameLayer.name).toBe('Alternate')
  })
  it('disables writes for locked glyphs', () => {
    setup(true)
    for (const button of screen.getAllByRole('button'))
      expect(button).toBeDisabled()
  })
  it('rejects an empty layer name', async () => {
    setup()
    await userEvent.clear(screen.getByLabelText(i18n.t('panel.layers.name')))
    for (const button of screen.getAllByRole('button'))
      expect(button).toBeDisabled()
  })
})
