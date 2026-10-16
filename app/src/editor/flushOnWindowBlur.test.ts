import { afterEach, expect, it, vi } from 'vitest'

import { installDraftFlushOnWindowBlur } from './flushOnWindowBlur'
import { registerDocumentEditParticipant } from './documentEditParticipants'
import type { DocId } from '../api'

const stops: (() => void)[] = []

afterEach(() => {
  while (stops.length > 0) stops.pop()?.()
})

function install(report = vi.fn()) {
  const stop = installDraftFlushOnWindowBlur(report)
  stops.push(stop)
  return report
}

it('persists a detached window draft when the window stops being active', async () => {
  const flush = vi.fn(async () => undefined)
  stops.push(registerDocumentEditParticipant(7 as DocId, { flush }))
  install()

  window.dispatchEvent(new Event('blur'))
  await vi.waitFor(() => expect(flush).toHaveBeenCalledTimes(1))

  window.dispatchEvent(new Event('pagehide'))
  await vi.waitFor(() => expect(flush).toHaveBeenCalledTimes(2))
})

it('persists drafts when the window is hidden rather than blurred', async () => {
  const flush = vi.fn(async () => undefined)
  stops.push(registerDocumentEditParticipant(8 as DocId, { flush }))
  install()
  const visibility = vi
    .spyOn(document, 'visibilityState', 'get')
    .mockReturnValue('hidden')

  document.dispatchEvent(new Event('visibilitychange'))
  await vi.waitFor(() => expect(flush).toHaveBeenCalledTimes(1))

  visibility.mockReturnValue('visible')
  document.dispatchEvent(new Event('visibilitychange'))
  expect(flush).toHaveBeenCalledTimes(1)
  visibility.mockRestore()
})

it('reports a failed flush instead of losing the rejection', async () => {
  const failure = new Error('kernel refused the draft')
  stops.push(
    registerDocumentEditParticipant(9 as DocId, {
      flush: vi.fn(async () => {
        throw failure
      }),
    }),
  )
  const report = install()

  window.dispatchEvent(new Event('blur'))

  await vi.waitFor(() => expect(report).toHaveBeenCalledWith(failure))
})

it('does nothing while no editor holds a draft', () => {
  const report = install()
  window.dispatchEvent(new Event('blur'))
  expect(report).not.toHaveBeenCalled()
})
