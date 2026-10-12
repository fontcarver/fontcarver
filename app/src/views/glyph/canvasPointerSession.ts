interface PointerEventSource {
  pointerId: number
  button: number
  buttons?: number
  isPrimary?: boolean
  currentTarget: HTMLCanvasElement
}

interface Callbacks {
  cancel: () => void
  space: (held: boolean) => void
  toggle: () => void
}

/** Owns one canvas pointer and temporary Space state across focus changes. */
export class CanvasPointerSession {
  private pointer: {
    id: number
    buttonMask: number
    canvas: HTMLCanvasElement
  } | null = null
  private held = false
  private used = false

  private callbacks: Callbacks = {
    cancel: () => {},
    space: () => {},
    toggle: () => {},
  }

  /** Bind current React callbacks after rendering, before handling input. */
  setCallbacks(callbacks: Callbacks): void {
    this.callbacks = callbacks
  }

  get active(): boolean {
    return this.pointer !== null
  }

  get spaceHeld(): boolean {
    return this.held
  }

  /** Only primary drawing and middle-button panning own a pointer. */
  down(event: PointerEventSource): boolean {
    if (
      this.pointer ||
      event.isPrimary === false ||
      (event.button !== 0 && event.button !== 1)
    )
      return false
    const canvas = event.currentTarget
    canvas.focus({ preventScroll: true })
    this.pointer = {
      id: event.pointerId,
      buttonMask: event.button === 1 ? 4 : 1,
      canvas,
    }
    if (this.held) this.used = true
    try {
      canvas.setPointerCapture(event.pointerId)
    } catch {
      // Synthetic events may not have a capturable pointer.
    }
    return true
  }

  /** Hover is allowed; another pointer cannot move an owned gesture. */
  move(
    event: Pick<PointerEventSource, 'pointerId' | 'isPrimary' | 'buttons'>,
  ): boolean {
    if (event.isPrimary === false) return false
    if (this.pointer && this.pointer.id !== event.pointerId) return false
    if (
      this.pointer &&
      event.buttons !== undefined &&
      (event.buttons & this.pointer.buttonMask) === 0
    ) {
      this.cancel()
      return false
    }
    if (this.pointer && this.held) this.used = true
    return true
  }

  /** Clear ownership before releasing capture, whose loss is then harmless. */
  up(event: Pick<PointerEventSource, 'pointerId'>): boolean {
    if (!this.pointer || this.pointer.id !== event.pointerId) return false
    this.release()
    return true
  }

  spaceDown(repeat: boolean): void {
    if (repeat || this.held) return
    this.held = true
    this.used = this.active
    this.callbacks.space(true)
  }

  spaceUp(): void {
    const toggle = this.held && !this.used
    this.held = false
    this.used = false
    this.callbacks.space(false)
    if (toggle) this.callbacks.toggle()
  }

  /** Abandon transient input without turning a lost Space release into a tap. */
  cancel(): void {
    const active = this.active
    this.held = false
    this.used = false
    this.callbacks.space(false)
    this.release()
    if (active) this.callbacks.cancel()
  }

  cancelPointer(event: Pick<PointerEventSource, 'pointerId'>): void {
    if (this.pointer?.id === event.pointerId) this.cancel()
  }

  private release(): void {
    const pointer = this.pointer
    this.pointer = null
    if (!pointer) return
    try {
      pointer.canvas.releasePointerCapture(pointer.id)
    } catch {
      // Cancellation can arrive after the browser has released capture.
    }
  }
}

/** Bind interruption events outside React's successful pointer-up path. */
export function bindCanvasPointerLifecycle(
  canvas: HTMLCanvasElement,
  session: CanvasPointerSession,
): () => void {
  const cancel = () => session.cancel()
  const pointer = (event: PointerEvent) => session.cancelPointer(event)
  const visibility = () => {
    if (document.visibilityState === 'hidden') cancel()
  }
  canvas.addEventListener('blur', cancel)
  canvas.addEventListener('pointercancel', pointer)
  canvas.addEventListener('lostpointercapture', pointer)
  window.addEventListener('blur', cancel)
  document.addEventListener('visibilitychange', visibility)
  return () => {
    canvas.removeEventListener('blur', cancel)
    canvas.removeEventListener('pointercancel', pointer)
    canvas.removeEventListener('lostpointercapture', pointer)
    window.removeEventListener('blur', cancel)
    document.removeEventListener('visibilitychange', visibility)
    cancel()
  }
}
