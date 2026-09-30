import { fireEvent, renderHook } from '@testing-library/react'
import { createRef } from 'react'
import { expect, it, vi } from 'vitest'
import { useTextTyping } from './useTextTyping'

it('reserves punctuation only for a selected locked kerning gap', () => {
  const change = vi.fn()
  const input = createRef<HTMLInputElement>()
  const hook = renderHook(
    ({ reserve }) => useTextTyping('AV', 'text', change, input, reserve),
    { initialProps: { reserve: true } },
  )
  fireEvent.keyDown(window, { key: '.' })
  fireEvent.keyDown(window, { key: ',' })
  expect(change).not.toHaveBeenCalled()
  fireEvent.keyDown(window, { key: 'A' })
  expect(change).toHaveBeenLastCalledWith('AVA')
  hook.rerender({ reserve: false })
  fireEvent.keyDown(window, { key: '.' })
  expect(change).toHaveBeenLastCalledWith('AV.')
})
