/** Keyboard sidebearing edits for the text view. */

import type { Command, DocId } from '../../api'
import { EditSession, type Scheduler } from '../../editor/editSession'

/** Idle time after which consecutive metrics keys become one undo entry. */
export const TEXT_METRICS_TIMEOUT = 500

/** The sidebearing changed by a metrics shortcut. */
export type MetricSide = 'lsb' | 'rsb'

/** Keyboard fields used by the shortcut mapper; DOM and React events conform. */
export interface MetricShortcutEvent {
  key: string
  altKey: boolean
  shiftKey: boolean
  ctrlKey: boolean
  metaKey: boolean
}

/** One recognized metrics key press. */
export interface MetricShortcut {
  side: MetricSide
  delta: number
}

/**
 * Maps Mod+Left/Right to sidebearing changes.
 *
 * macOS can distinguish Meta-as-Mod plus Ctrl for the 100-unit step. On Linux,
 * Ctrl is Mod itself, so Ctrl+arrow deliberately remains the one-unit step.
 * Alt combinations are reserved for future kerning commands.
 */
export function metricShortcut(
  event: MetricShortcutEvent,
  isMac: boolean,
): MetricShortcut | null {
  if (event.altKey) return null
  const side =
    event.key === 'ArrowLeft'
      ? 'lsb'
      : event.key === 'ArrowRight'
        ? 'rsb'
        : null
  const mod = isMac ? event.metaKey : event.ctrlKey
  if (!side || !mod) return null

  const step = isMac && event.ctrlKey ? 100 : event.shiftKey ? 10 : 1
  return { side, delta: side === 'lsb' ? -step : step }
}

/** Horizontal metrics at the start of a keyboard run. */
export interface TextMetrics {
  width: number
  lsb: number
  rsb: number
}

/** Optimistic metrics shown while the kernel edit is open. */
export interface TextMetricsPreview extends TextMetrics {
  /** Translation applied to the active glyph's geometry. */
  lsbDelta: number
}

/** Dependencies and callbacks for one coalesced keyboard metrics run. */
export interface TextMetricsRunnerOptions {
  doc: DocId
  glyph: string
  layer: string
  start: TextMetrics
  /** Whether sidebearing edits have geometry bounds to move. Defaults to true. */
  hasOutline?: boolean
  /** Undo label, already translated. */
  label: string
  /** A controller edit that this run must not overtake. */
  after?: Promise<unknown>
  scheduler?: Scheduler
  /** Idle timeout injected by tests; production uses 500 ms. */
  timeout?: number
  onPreview: (preview: TextMetricsPreview) => void
  /** Called only after `doc.edit_end` has settled. */
  onEnd: () => void
  onError: (error: unknown) => void
}

/** One 500 ms run of absolute `SetMetrics` commands. */
export class TextMetricsRunner {
  private readonly options: TextMetricsRunnerOptions
  private session: EditSession | null = null
  private ending: Promise<void> | null = null
  private timer: ReturnType<typeof setTimeout> | null = null
  private lsbDelta = 0
  private rsbDelta = 0

  constructor(options: TextMetricsRunnerOptions) {
    this.options = options
  }

  /** Whether this runner currently owns an open EditSession. */
  get active(): boolean {
    return this.session !== null
  }

  /** Adds one absolute sidebearing step and publishes its optimistic result. */
  press(side: MetricSide, delta: number): void {
    if (delta === 0 || this.ending) return
    if (!this.session) {
      this.session = new EditSession(this.options.doc, this.options.label, {
        after: this.options.after,
        scheduler: this.options.scheduler,
        onError: this.options.onError,
      })
    }

    if (side === 'lsb') this.lsbDelta += delta
    else this.rsbDelta += delta
    this.options.onPreview(this.preview())
    this.session.queue(() => this.command())
    this.restartTimer()
  }

  /** Finalises the run once and resolves after the kernel has closed it. */
  end(): Promise<void> {
    if (this.ending) return this.ending
    const session = this.session
    if (!session) return Promise.resolve()
    this.session = null
    if (this.timer !== null) clearTimeout(this.timer)
    this.timer = null

    this.ending = session.end().then(() => {
      this.options.onEnd()
    })
    return this.ending
  }

  private restartTimer(): void {
    if (this.timer !== null) clearTimeout(this.timer)
    this.timer = setTimeout(() => {
      this.timer = null
      void this.end().catch(this.options.onError)
    }, this.options.timeout ?? TEXT_METRICS_TIMEOUT)
  }

  private preview(): TextMetricsPreview {
    const { start } = this.options
    if (this.options.hasOutline === false) {
      const width = start.width + this.lsbDelta + this.rsbDelta
      return { width, lsb: 0, rsb: width, lsbDelta: 0 }
    }
    return {
      width: start.width + this.lsbDelta + this.rsbDelta,
      lsb: start.lsb + this.lsbDelta,
      rsb: start.rsb + this.rsbDelta,
      lsbDelta: this.lsbDelta,
    }
  }

  private command(): Command {
    const preview = this.preview()
    const hasOutline = this.options.hasOutline !== false
    return {
      SetMetrics: {
        glyph: this.options.glyph,
        layer: this.options.layer,
        width: hasOutline ? null : preview.width,
        lsb: hasOutline ? preview.lsb : null,
        rsb: hasOutline ? preview.rsb : null,
      },
    }
  }
}
