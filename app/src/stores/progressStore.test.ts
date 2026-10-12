import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import { setTransport } from '../api'
import { createMockTransport } from '../test/mockTransport'

import {
  initialProgressState,
  selectProgress,
  useProgressStore,
} from './progressStore'
import { initialUiState, useUiStore } from './uiStore'

function progressEvent(stage: string, percent: number, message?: string) {
  return { kind: 'Progress' as const, doc: 1, stage, percent, message }
}

beforeEach(() => {
  localStorage.clear()
  useUiStore.setState(initialUiState())
  useProgressStore.setState(initialProgressState())
})

afterEach(() => {
  setTransport(undefined)
})

describe('progressStore', () => {
  it('tracks the stages of one operation', () => {
    const transport = createMockTransport()
    setTransport(transport)
    const stop = useProgressStore.getState().start()

    transport.emit(progressEvent('compile', 10))
    expect(useProgressStore.getState().byDoc[1]).toEqual({
      stage: 'compile',
      percent: 10,
    })

    transport.emit(progressEvent('preserve', 70))
    expect(useProgressStore.getState().byDoc[1]).toEqual({
      stage: 'preserve',
      percent: 70,
    })

    stop()
  })

  it('clears the line when the operation finishes', () => {
    const transport = createMockTransport()
    setTransport(transport)
    const stop = useProgressStore.getState().start()

    transport.emit(progressEvent('compile', 10))
    transport.emit(progressEvent('write', 100))

    expect(useProgressStore.getState().byDoc[1]).toBeUndefined()
    stop()
  })

  it('reports a failure as an error toast and ends the line', () => {
    const transport = createMockTransport()
    setTransport(transport)
    const stop = useProgressStore.getState().start()

    transport.emit(progressEvent('compile', 40))
    transport.emit(progressEvent('failed', 100, 'disk is full'))

    expect(useProgressStore.getState().byDoc[1]).toBeUndefined()
    expect(useUiStore.getState().toasts).toEqual([
      { id: expect.any(String), kind: 'error', message: 'disk is full' },
    ])
    stop()
  })

  it('shows a line as soon as an operation is requested', () => {
    useProgressStore.getState().begin(7)

    expect(useProgressStore.getState().byDoc[7]).toEqual({
      stage: 'compile',
      percent: 0,
    })

    useProgressStore.getState().clear(7)
    expect(useProgressStore.getState().byDoc[7]).toBeUndefined()
  })

  it('prefers the active document but falls back to any running operation', () => {
    useProgressStore.getState().begin(9)

    expect(selectProgress(1)(useProgressStore.getState())).toEqual({
      stage: 'compile',
      percent: 0,
    })

    useProgressStore.getState().begin(1)
    useProgressStore.setState((state) => ({
      byDoc: { ...state.byDoc, 1: { stage: 'write', percent: 55 } },
    }))
    expect(selectProgress(1)(useProgressStore.getState())).toEqual({
      stage: 'write',
      percent: 55,
    })
  })
})
