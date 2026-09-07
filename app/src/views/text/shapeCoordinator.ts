import type { DocumentEvent } from '../../api'
import type { Transport } from '../../transport'

import {
  buildTextLayout,
  type TextLayout,
  type TextRequestContext,
} from './textLayout'

export type { TextRequestContext } from './textLayout'

export interface TextShapeState {
  context: TextRequestContext
  layout: TextLayout
  stale: boolean
  showingPrevious: boolean
  compileFailure: string | null
  /** Set only for the first observation of this failure in this request. */
  newCompileFailure: string | null
}

export interface TextShapeCoordinatorOptions {
  debounceMs?: number
  pollDelaysMs?: readonly number[]
  onError?: (error: unknown) => void
}

const DEFAULT_POLL_DELAYS = [100, 250, 500, 1_000] as const

function requestKey(context: TextRequestContext): string {
  return JSON.stringify([
    context.doc,
    context.master,
    context.text,
    context.features,
    context.direction,
    context.script,
    context.language,
    context.unitsPerEm,
  ])
}

/**
 * Debounces shaping and owns stale-result polling. The coordinator is unaware
 * of editor/controller state; integration forwards committed document events
 * through `notifyDocumentEvent` and disposes it on unmount.
 */
export class TextShapeCoordinator {
  private readonly debounceMs: number
  private readonly pollDelaysMs: readonly number[]
  private readonly onError: (error: unknown) => void
  private timer: ReturnType<typeof setTimeout> | null = null
  private epoch = 0
  private disposed = false
  private context: TextRequestContext | null = null
  private lastGood: { key: string; layout: TextLayout } | null = null
  private readonly reportedFailures = new Set<string>()

  constructor(
    private readonly transport: Transport,
    private readonly onState: (state: TextShapeState) => void,
    options: TextShapeCoordinatorOptions = {},
  ) {
    this.debounceMs = options.debounceMs ?? 150
    this.pollDelaysMs =
      options.pollDelaysMs && options.pollDelaysMs.length > 0
        ? options.pollDelaysMs
        : DEFAULT_POLL_DELAYS
    this.onError = options.onError ?? (() => undefined)
  }

  /** Replaces the request context and starts a new 150 ms typing debounce. */
  update(context: TextRequestContext): void {
    if (this.disposed) return
    this.context = {
      ...context,
      features: context.features.map(([tag, enabled]) => [tag, enabled]),
    }
    this.schedule(this.debounceMs)
  }

  /** Debounces relevant document events so live editing refreshes after it ends. */
  notifyDocumentEvent(event: DocumentEvent): void {
    if (event.doc !== this.context?.doc || this.disposed) return
    this.context = { ...this.context, generation: event.generation }
    this.schedule(this.debounceMs)
  }

  dispose(): void {
    this.disposed = true
    this.epoch += 1
    this.clearTimer()
  }

  private clearTimer(): void {
    if (this.timer !== null) clearTimeout(this.timer)
    this.timer = null
  }

  private schedule(delay: number, pollAttempt = 0): void {
    this.clearTimer()
    const epoch = ++this.epoch
    this.timer = setTimeout(() => {
      this.timer = null
      void this.execute(epoch, pollAttempt)
    }, delay)
  }

  private async execute(epoch: number, pollAttempt: number): Promise<void> {
    const context = this.context
    if (!context || this.disposed) return
    const key = requestKey(context)
    try {
      const layout = await buildTextLayout(this.transport, context)
      if (this.disposed || epoch !== this.epoch) return

      const compileFailure =
        layout.diagnostics.find((item) => item.code === 'compile-failed')
          ?.message ?? null
      const failureKey = compileFailure
        ? `${context.doc}\0${context.generation}\0${compileFailure}`
        : null
      const newCompileFailure =
        failureKey && !this.reportedFailures.has(failureKey)
          ? compileFailure
          : null
      if (failureKey) {
        this.reportedFailures.add(failureKey)
        while (this.reportedFailures.size > 64) {
          const oldest = this.reportedFailures.values().next()
          if (oldest.done) break
          this.reportedFailures.delete(oldest.value)
        }
      }

      const previous = this.lastGood?.key === key ? this.lastGood.layout : null
      if (!layout.stale) this.lastGood = { key, layout }
      this.onState({
        context,
        layout: layout.stale && previous ? previous : layout,
        stale: layout.stale,
        showingPrevious: layout.stale && previous !== null,
        compileFailure,
        newCompileFailure,
      })

      if (layout.stale && compileFailure === null) {
        const index = Math.min(pollAttempt, this.pollDelaysMs.length - 1)
        this.schedule(this.pollDelaysMs[index] ?? 1_000, pollAttempt + 1)
      }
    } catch (error) {
      if (!this.disposed && epoch === this.epoch) this.onError(error)
    }
  }
}
