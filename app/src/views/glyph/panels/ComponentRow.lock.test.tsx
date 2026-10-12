import { fireEvent, render, screen } from '@testing-library/react'
import { expect, it, vi } from 'vitest'
import { i18n } from '../../../i18n'
import { ComponentRow } from './ComponentRow'
import { newComponent } from './panelCommands'

it('keeps locked components inspectable and permits only unlocking', () => {
  const onWrite = vi.fn()
  const component = { ...newComponent('acute'), locked: true }
  render(
    <ComponentRow
      view={{ component, path: '', bbox: null }}
      index={0}
      count={2}
      selected
      dragging={false}
      onSelect={vi.fn()}
      onOpenBase={vi.fn()}
      onWrite={onWrite}
      onRemove={vi.fn()}
      onDecompose={vi.fn()}
      onMove={vi.fn()}
      onDragStart={vi.fn()}
      onDragEnd={vi.fn()}
    />,
  )
  expect(
    screen.getByRole('button', {
      name: i18n.t('panel.components.remove', { name: 'acute' }),
    }),
  ).toBeDisabled()
  expect(
    screen.getByRole('button', {
      name: i18n.t('panel.components.decompose', { name: 'acute' }),
    }),
  ).toBeDisabled()
  expect(screen.getByLabelText(i18n.t('infoBox.x'))).toBeDisabled()
  expect(
    screen.getByRole('button', {
      name: i18n.t('panel.components.openBase', { name: 'acute' }),
    }),
  ).toBeEnabled()
  fireEvent.click(
    screen.getByRole('button', { name: i18n.t('panel.components.lock') }),
  )
  expect(onWrite).toHaveBeenCalledWith({ ...component, locked: false })
})
