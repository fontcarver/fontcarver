import { call, type Command, type DocId, type KernSide } from '../../api'
import {
  EditSession,
  holdDocumentEdits,
  type Scheduler,
} from '../../editor/editSession'
import { ApiCallError } from '../../transport'
import {
  kerningScopeRecord,
  kerningScopeSides,
  type KerningPairNames,
  type KerningScope,
} from './kerningModel'

/** Modifier subset shared by React and native canvas events. */
export interface KerningShortcutEvent {
  key: string
  code?: string
  altKey: boolean
  ctrlKey: boolean
  metaKey: boolean
  shiftKey: boolean
  target?: EventTarget | null
}

/**
 * The kerning gesture for one key press, including held class suppression.
 *
 * `punctuation` enables the `,`/`.` kerning keys at all; `bare` additionally
 * allows them **unmodified**, which only the locked kerning mode can afford
 * because otherwise those characters are typed into the run. A held Alt or
 * Control is never typed, so a modified punctuation edit is available in every
 * kerning mode and is how an exception is made without changing the panel's
 * scope.
 *
 * Alt suppresses the second side's class on every platform. Control suppresses
 * the first side's class on macOS only: everywhere else Control *is* the
 * primary modifier, so claiming it here would fight the user's own `Mod+,`
 * binding. The scope selector covers that case on those platforms.
 */
export function kerningShortcutAction(
  event: KerningShortcutEvent,
  isMac: boolean,
  punctuation = false,
  bare = true,
): { delta: number; suppressLeft: boolean; suppressRight: boolean } | null {
  const arrows = kerningShortcut(event, isMac)
  if (arrows !== null)
    return { delta: arrows, suppressLeft: false, suppressRight: false }
  const mod = isMac ? event.metaKey : event.ctrlKey
  if (!punctuation || mod || event.metaKey) return null
  const direction =
    event.code === 'Comma' || [',', '<'].includes(event.key)
      ? -1
      : event.code === 'Period' || ['.', '>'].includes(event.key)
        ? 1
        : null
  if (direction === null) return null
  const suppressLeft = event.ctrlKey
  const suppressRight = event.altKey
  // An unmodified comma or period is a character the text run wants.
  if (!bare && !suppressLeft && !suppressRight) return null
  return {
    delta: direction * (event.shiftKey ? 10 : 1),
    suppressLeft,
    suppressRight,
  }
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
  vertical?: boolean
  master: string
  left: KernSide
  right: KernSide
}

/** Validates captured class membership after earlier queued edits finish. */
export async function refreshKerningBaseline(
  target: KerningEditTarget,
  pair: KerningPairNames,
  scope: KerningScope,
): Promise<number> {
  const fresh = await call(
    target.vertical ? 'kerning.resolve_vertical' : 'kerning.resolve',
    {
      doc: target.doc,
      master: target.master,
      ...pair,
    },
  )
  if (!fresh.status.editable || fresh.value === null)
    throw new ApiCallError('kerning_unavailable', 'Kerning is unavailable')
  const sides = kerningScopeSides(fresh, scope)
  if (
    !sides ||
    JSON.stringify(sides.left) !== JSON.stringify(target.left) ||
    JSON.stringify(sides.right) !== JSON.stringify(target.right)
  )
    throw new ApiCallError(
      'stale_generation',
      'Kerning group membership changed',
    )
  return kerningScopeRecord(fresh, scope)?.value ?? fresh.value
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
    return this.options.vertical
      ? { SetVerticalKerning: { master, left, right, value } }
      : { SetHorizontalKerning: { master, left, right, value } }
  }
}
