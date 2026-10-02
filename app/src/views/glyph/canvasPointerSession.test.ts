import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  CanvasPointerSession,
  bindCanvasPointerLifecycle,
} from './canvasPointerSession'

const disposers: (() => void)[] = []
afterEach(() => {
  for (const dispose of disposers.splice(0)) dispose()
  document.body.replaceChildren()
  vi.restoreAllMocks()
})

function setup() {
  const canvas = document.createElement('canvas')
  canvas.tabIndex = 0
  document.body.append(canvas)
  canvas.setPointerCapture = vi.fn()
  canvas.releasePointerCapture = vi.fn()
  const callbacks = { cancel: vi.fn(), space: vi.fn(), toggle: vi.fn() }
  const session = new CanvasPointerSession()
  session.setCallbacks(callbacks)
  disposers.push(bindCanvasPointerLifecycle(canvas, session))
  const event = (pointerId = 1, button = 0) => ({
    pointerId,
    button,
    currentTarget: canvas,
  })
  return { canvas, callbacks, session, event }
}

describe('canvas pointer ownership', () => {
  it('rejects secondary buttons without capture or focus changes', () => {
    const { canvas, session, event } = setup()
    expect(session.down(event(1, 2))).toBe(false)
    expect(canvas.setPointerCapture).not.toHaveBeenCalled()
    expect(document.activeElement).not.toBe(canvas)
  })

  it('accepts middle pan and ignores unrelated pointer events', () => {
    const { canvas, session, event } = setup()
    expect(session.down(event(1, 1))).toBe(true)
    expect(session.down(event(2))).toBe(false)
    expect(session.move(event(2))).toBe(false)
    expect(session.up(event(2))).toBe(false)
    expect(session.up(event(1, 1))).toBe(true)
    expect(canvas.releasePointerCapture).toHaveBeenCalledExactlyOnceWith(1)
  })

  it.each([0, 1])(
    'cancels button %s when a chord releases its initiating button first',
    (button) => {
      const { callbacks, session, event } = setup()
      session.down(event(1, button))
      expect(session.move({ ...event(), buttons: button === 1 ? 6 : 3 })).toBe(
        true,
      )
      expect(session.move({ ...event(), buttons: 2 })).toBe(false)
      expect(callbacks.cancel).toHaveBeenCalledOnce()
      expect(session.active).toBe(false)
      expect(session.up(event(1, 2))).toBe(false)
    },
  )

  it('aborts cancelled captures but ignores loss after a successful release', () => {
    const { canvas, callbacks, session, event } = setup()
    session.down(event())
    const cancel = new Event('pointercancel')
    Object.defineProperty(cancel, 'pointerId', { value: 1 })
    canvas.dispatchEvent(cancel)
    expect(callbacks.cancel).toHaveBeenCalledOnce()
    expect(session.up(event())).toBe(false)
    session.down(event())
    expect(session.up(event())).toBe(true)
    const lost = new Event('lostpointercapture')
    Object.defineProperty(lost, 'pointerId', { value: 1 })
    canvas.dispatchEvent(lost)
    expect(callbacks.cancel).toHaveBeenCalledOnce()
  })

  it.each(['canvas', 'window'])(
    'clears held Space and aborts on %s blur without toggling fill',
    (target) => {
      const { canvas, callbacks, session, event } = setup()
      session.down(event())
      session.spaceDown(false)
      ;(target === 'canvas' ? canvas : window).dispatchEvent(new Event('blur'))
      expect(session.spaceHeld).toBe(false)
      expect(session.active).toBe(false)
      expect(callbacks.space).toHaveBeenLastCalledWith(false)
      expect(callbacks.cancel).toHaveBeenCalledOnce()
      session.spaceUp()
      expect(callbacks.toggle).not.toHaveBeenCalled()
    },
  )

  it('distinguishes a Space tap from Space used during an existing pen press', () => {
    const { callbacks, session, event } = setup()
    session.spaceDown(false)
    session.spaceUp()
    expect(callbacks.toggle).toHaveBeenCalledOnce()
    session.down(event())
    session.spaceDown(false)
    session.move(event())
    session.spaceUp()
    expect(callbacks.toggle).toHaveBeenCalledOnce()
  })

  it('releases Space without discarding a pen draft between pointer presses', () => {
    const { canvas, callbacks, session } = setup()
    session.spaceDown(false)
    canvas.dispatchEvent(new Event('blur'))
    expect(session.spaceHeld).toBe(false)
    expect(callbacks.cancel).not.toHaveBeenCalled()
    session.spaceUp()
    expect(callbacks.toggle).not.toHaveBeenCalled()
  })
})
