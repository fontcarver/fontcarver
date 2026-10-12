/** Keyboard sidebearing edits for the text view. */

import type { Command, DocId } from '../../api'
import { EditSession, type Scheduler } from '../../editor/editSession'

/** Idle time after which consecutive metrics keys become one undo entry. */
export const TEXT_METRICS_TIMEOUT = 500

/** The sidebearing changed by a metrics shortcut. */
export type MetricSide = 'lsb' | 'rsb' | 'both'

/** Keyboard fields used by the shortcut mapper; DOM and React events conform. */
export interface MetricShortcutEvent {
  key: string
  code?: string
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

/** Selected side and configured base step for bracket spacing shortcuts. */
export interface MetricShortcutOptions {
  side: MetricSide
  /** Positive font-unit step; missing or invalid values use one unit. */
  step?: number
}

/**
 * Maps Mod+Left/Right to legacy sidebearing changes. Optional selected-side
 * brackets use the configured step, ×10 with Shift, ×100 with Alt, and exactly
 * one font unit with Alt+Shift.
 *
 * `Mod+[` and `Mod+]` are the shell's previous/next glyph commands, so a
 * Ctrl/Meta bracket is not a spacing step: pressing it used to move the glyph
 * being left by 100 units as the tab switched. macOS can distinguish
 * Meta-as-Mod plus Ctrl for the 100-unit arrow step. On Linux, Ctrl is Mod
 * itself, so Ctrl+arrow deliberately remains the one-unit step. Alt arrows
 * remain reserved for kerning commands.
 */
export function metricShortcut(
  event: MetricShortcutEvent,
  isMac: boolean,
  options?: MetricShortcutOptions,
): MetricShortcut | null {
  const mod = isMac ? event.metaKey : event.ctrlKey
  if (options) {
    const direction =
      event.code === 'BracketLeft' || event.key === '[' || event.key === '{'
        ? -1
        : event.code === 'BracketRight' ||
            event.key === ']' ||
            event.key === '}'
          ? 1
          : 0
    // Never claim navigation or Ctrl+Alt (AltGr) bracket combinations.
    if (direction && !event.ctrlKey && !event.metaKey) {
      const configured = options.step ?? 1
      const base =
        Number.isFinite(configured) && configured > 0 ? configured : 1
      const step = event.altKey
        ? event.shiftKey
          ? 1
          : base * 100
        : base * (event.shiftKey ? 10 : 1)
      return { side: options.side, delta: direction * step }
    }
    if (direction) return null
  }
  if (event.altKey) return null
  const side =
    event.key === 'ArrowLeft'
      ? 'lsb'
      : event.key === 'ArrowRight'
        ? 'rsb'
        : null
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
  /** Refresh the baseline after prior edits and edit_begin finish. */
  refreshStart?: () => Promise<{ metrics: TextMetrics; hasOutline: boolean }>
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
  private baseline: TextMetrics
  private hasOutline: boolean
  private preparing: Promise<void> | null = null
  private baselineReady = true

  constructor(options: TextMetricsRunnerOptions) {
    this.options = options
    this.baseline = { ...options.start }
    this.hasOutline = options.hasOutline !== false
  }

  /** Whether this runner currently owns an open EditSession. */
  get active(): boolean {
    return this.session !== null
  }

  /** Adds one absolute sidebearing step and publishes its optimistic result. */
  press(side: MetricSide, delta: number): void {
    if (delta === 0 || this.ending) return
    if (!this.session) {
      const session = new EditSession(this.options.doc, this.options.label, {
        after: this.options.after,
        scheduler: this.options.scheduler,
        onError: this.options.onError,
      })
      this.session = session
      const refresh = this.options.refreshStart
      if (refresh) {
        this.baselineReady = false
        this.preparing = session
          .ready()
          .then(refresh)
          .then((fresh) => {
            this.baseline = { ...fresh.metrics }
            this.hasOutline = fresh.hasOutline
            this.baselineReady = true
            this.options.onPreview(this.preview())
            session.queue(() => this.command())
          })
          .catch((error: unknown) => {
            if (!session.failed) this.options.onError(error)
          })
      }
    }

    if (side === 'lsb' || side === 'both') this.lsbDelta += delta
    if (side === 'rsb' || side === 'both') this.rsbDelta += delta
    if (this.baselineReady) {
      this.options.onPreview(this.preview())
      this.session.queue(() => this.command())
    }
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

    // The session was constructed synchronously, before callers publish this
    // close as a document barrier. Waiting for the fresh baseline here cannot
    // make its own edit_begin depend on a future closure.
    const closing = this.preparing
      ? this.preparing.then(() => session.end())
      : session.end()
    this.ending = closing.then(() => this.options.onEnd())
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
    const start = this.baseline
    if (!this.hasOutline) {
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
    const hasOutline = this.hasOutline
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
