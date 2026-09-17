import type { Command, DocId, KernSide } from '../../api'
import {
  EditSession,
  holdDocumentEdits,
  type Scheduler,
} from '../../editor/editSession'
import { ApiCallError } from '../../transport'

/** Modifier subset shared by React and native canvas events. */
export interface KerningShortcutEvent {
  key: string
  altKey: boolean
  ctrlKey: boolean
  metaKey: boolean
  shiftKey: boolean
  target?: EventTarget | null
}

/** Reserved horizontal kerning keys, with platform-specific Mod semantics. */
export function kerningShortcut(
  event: KerningShortcutEvent,
  isMac: boolean,
): number | null {
  const mod = isMac ? event.metaKey : event.ctrlKey
  if (!mod || !event.altKey || !['ArrowLeft', 'ArrowRight'].includes(event.key))
    return null
  const step = isMac && event.ctrlKey ? 100 : event.shiftKey ? 10 : 1
  return event.key === 'ArrowLeft' ? -step : step
}

/** Immutable rule identity captured when an edit starts. */
export interface KerningEditTarget {
  doc: DocId
  master: string
  left: KernSide
  right: KernSide
}

/** Callbacks for a single coalesced kerning command session. */
export interface KerningEditOptions extends KerningEditTarget {
  label: string
  after?: Promise<unknown>
  scheduler?: Scheduler
  refreshStart: () => Promise<number>
  onValue?: (value: number | null) => void
  onEnd: () => void
  onError: (error: unknown) => void
}

/** Applies absolute stored values; shaped positions always come from the kernel. */
export class KerningEditRunner {
  private session: EditSession | null = null
  private ending: Promise<void> | null = null
  private preparing: Promise<void> | null = null
  private timer: ReturnType<typeof setTimeout> | null = null
  private delta = 0
  private baseline: number | null = null

  constructor(private readonly options: KerningEditOptions) {}

  get active(): boolean {
    return this.session !== null
  }

  /** Accumulates keys while an earlier document edit or baseline read finishes. */
  press(delta: number): void {
    if (!Number.isFinite(delta) || delta === 0 || this.ending) return
    const session = this.begin()
    this.delta += delta
    if (!this.preparing) {
      this.preparing = session
        .ready()
        .then(this.options.refreshStart)
        .then((value) => {
          if (!Number.isFinite(value))
            throw new ApiCallError('invalid_params', 'Invalid kerning baseline')
          this.baseline = value
          this.queueDelta(session)
        })
        .catch((error: unknown) => {
          if (!session.failed) this.options.onError(error)
        })
    } else if (this.baseline !== null) this.queueDelta(session)
    if (this.timer !== null) clearTimeout(this.timer)
    this.timer = setTimeout(() => {
      void this.end()
    }, 500)
  }

  /** Sets an explicit value, including zero; null removes only this record. */
  set(value: number | null): void {
    if (this.ending || (value !== null && !Number.isFinite(value))) return
    const session = this.begin()
    this.preparing = session
      .ready()
      .then(this.options.refreshStart)
      .then(() => {
        session.queue(() => this.command(value))
        this.options.onValue?.(value)
      })
      .catch((error: unknown) => {
        if (!session.failed) this.options.onError(error)
      })
  }

  /** Flushes exactly once and publishes the closure to document transitions. */
  end(): Promise<void> {
    if (this.ending) return this.ending
    const session = this.session
    if (!session) return Promise.resolve()
    this.session = null
    if (this.timer !== null) clearTimeout(this.timer)
    this.timer = null
    const closing = (this.preparing ?? Promise.resolve())
      .then(() => session.end())
      .then(this.options.onEnd)
    this.ending = holdDocumentEdits(this.options.doc, closing)
    return this.ending
  }

  private begin(): EditSession {
    this.session ??= new EditSession(
      this.options.doc,
      this.options.label,
      this.options,
    )
    return this.session
  }

  private queueDelta(session: EditSession): void {
    if (this.baseline === null) return
    const value = this.baseline + this.delta
    this.options.onValue?.(value)
    session.queue(() => this.command(value))
  }

  private command(value: number | null): Command {
    const { master, left, right } = this.options
    return { SetHorizontalKerning: { master, left, right, value } }
  }
}
