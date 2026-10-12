import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it, vi } from 'vitest'
import type { Component, SmartAxis } from '../../../api'
import { SmartComponentControls } from './SmartComponentControls'

const axes: SmartAxis[] = [
  { name: 'Width', min: 0, default: 50, max: 100 },
  { name: 'Height', min: -100, default: 0, max: 100 },
  { name: 'Weight', min: 1, default: 2, max: 5 },
]
const component: Component = {
  base: 'part',
  transform: [1, 0, 0, 1, 20, 30],
  auto_align: false,
  use_my_metrics: false,
  point_match: null,
  anchor: null,
  round_to_grid: false,
  smart: { Height: 25, Weight: 3 },
}

it('shows default parameters and commits a bounded slider gesture once', () => {
  const onWrite = vi.fn()
  render(
    <SmartComponentControls
      axes={axes}
      component={component}
      onWrite={onWrite}
    />,
  )
  const slider = screen.getByRole('slider', { name: 'Width' })
  expect(slider).toHaveValue('50')
  expect(slider).toHaveAttribute('min', '0')
  expect(slider).toHaveAttribute('max', '100')
  fireEvent.pointerDown(slider, { pointerId: 1 })
  fireEvent.change(slider, { target: { value: '75' } })
  expect(onWrite).not.toHaveBeenCalled()
  fireEvent.pointerUp(slider, { pointerId: 1 })
  fireEvent.blur(slider)
  expect(onWrite).toHaveBeenCalledExactlyOnceWith({
    ...component,
    smart: { Width: 75, Height: 25, Weight: 3 },
  })
})

it('cancels a slider draft and reconciles a rejected edit to the model', () => {
  const onWrite = vi.fn()
  const { rerender } = render(
    <SmartComponentControls
      axes={axes}
      component={component}
      onWrite={onWrite}
    />,
  )
  const slider = screen.getByRole('slider', { name: 'Width' })
  fireEvent.change(slider, { target: { value: '90' } })
  fireEvent.keyDown(slider, { key: 'Escape' })
  fireEvent.blur(slider)
  expect(onWrite).not.toHaveBeenCalled()
  expect(slider).toHaveValue('50')
  fireEvent.change(slider, { target: { value: '80' } })
  fireEvent.keyUp(slider, { key: 'ArrowRight' })
  expect(onWrite).toHaveBeenCalledOnce()
  rerender(
    <SmartComponentControls
      axes={axes}
      component={{ ...component }}
      onWrite={onWrite}
    />,
  )
  expect(slider).toHaveValue('50')
})

it('clamps numeric edits and resets local values while retaining the component transform', async () => {
  const onWrite = vi.fn()
  const user = userEvent.setup()
  render(
    <SmartComponentControls
      axes={axes}
      component={component}
      onWrite={onWrite}
    />,
  )
  const numeric = screen.getByRole('textbox', { name: 'Width' })
  await user.clear(numeric)
  await user.type(numeric, '150{Enter}')
  expect(onWrite).toHaveBeenLastCalledWith({
    ...component,
    smart: { Width: 100, Height: 25, Weight: 3 },
  })
  await user.click(
    screen.getByRole('button', { name: 'Reset smart parameters' }),
  )
  expect(onWrite).toHaveBeenLastCalledWith({ ...component, smart: null })
})

it('uses an accessible two-axis handle and preserves other local values', () => {
  const onWrite = vi.fn()
  render(
    <SmartComponentControls
      axes={axes}
      component={component}
      onWrite={onWrite}
    />,
  )
  const handle = screen.getByRole('button', { name: 'Adjust Width and Height' })
  fireEvent.keyDown(handle, { key: 'ArrowRight' })
  fireEvent.keyDown(handle, { key: 'ArrowUp', shiftKey: true })
  expect(onWrite).not.toHaveBeenCalled()
  fireEvent.keyUp(handle, { key: 'ArrowUp' })
  expect(onWrite).toHaveBeenCalledExactlyOnceWith({
    ...component,
    smart: { Width: 51, Height: 45, Weight: 3 },
  })
})

it('commits one two-axis pointer gesture, clamps outside the pad and cancels on Escape', () => {
  const onWrite = vi.fn()
  render(
    <SmartComponentControls
      axes={axes}
      component={component}
      onWrite={onWrite}
    />,
  )
  const pad = screen.getByRole('group', { name: 'Smart parameter handle' })
  vi.spyOn(pad, 'getBoundingClientRect').mockReturnValue({
    left: 10,
    top: 20,
    width: 100,
    height: 100,
    right: 110,
    bottom: 120,
    x: 10,
    y: 20,
    toJSON: () => ({}),
  })
  fireEvent.pointerDown(pad, {
    pointerId: 1,
    clientX: 85,
    clientY: 45,
    button: 0,
  })
  fireEvent.pointerMove(pad, { pointerId: 1, clientX: 210, clientY: -20 })
  expect(onWrite).not.toHaveBeenCalled()
  fireEvent.pointerUp(pad, { pointerId: 1 })
  expect(onWrite).toHaveBeenCalledExactlyOnceWith({
    ...component,
    smart: { Width: 100, Height: 100, Weight: 3 },
  })
  fireEvent.pointerDown(pad, {
    pointerId: 2,
    clientX: 20,
    clientY: 100,
    button: 0,
  })
  fireEvent.keyDown(
    screen.getByRole('button', { name: 'Adjust Width and Height' }),
    { key: 'Escape' },
  )
  fireEvent.pointerUp(pad, { pointerId: 2 })
  expect(onWrite).toHaveBeenCalledOnce()
})

it('disables parameter sliders, numbers, handle and reset for a locked component', () => {
  render(
    <SmartComponentControls
      axes={axes}
      component={component}
      disabled
      onWrite={vi.fn()}
    />,
  )
  expect(screen.getByRole('slider', { name: 'Width' })).toBeDisabled()
  expect(screen.getByRole('textbox', { name: 'Width' })).toBeDisabled()
  expect(
    screen.getByRole('button', { name: 'Adjust Width and Height' }),
  ).toBeDisabled()
  expect(
    screen.getByRole('button', { name: 'Reset smart parameters' }),
  ).toBeDisabled()
})
