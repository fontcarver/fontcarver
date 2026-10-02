import { call, type BitmapGlyph, type BitmapGlyphParams } from '../../api'
import {
  hasPendingDocumentDraft,
  registerDocumentEditParticipant,
} from '../../editor/documentEditParticipants'
import { noteDocumentEdit } from '../../editor/editSession'
export interface BitmapDraft {
  value: BitmapGlyph | null
  /** Flattened kernel preview; never use this as the editable source. */
  resolved: BitmapGlyph | null
  generation: number
  dirty: boolean
  busy: boolean
  suspended: boolean
  error: unknown
}
/** One persistent editor draft; failed writes keep their pixels and block handover. */
export class BitmapSession {
  private state: BitmapDraft = {
    value: null,
    resolved: null,
    generation: 0,
    dirty: false,
    busy: false,
    suspended: false,
    error: null,
  }
  private listeners = new Set<() => void>()
  private request = 0
  private pending: Promise<void> | undefined
  private gestureSource: BitmapDraft | undefined
  private acceptGesture: (() => void) | undefined
  private refreshAfterGesture = false
  constructor(
    readonly target: BitmapGlyphParams,
    readonly empty: BitmapGlyph,
  ) {}
  connect(): () => void {
    return registerDocumentEditParticipant(this.target.doc, {
      flush: () => this.flush(),
      hasPendingEdits: () => this.state.dirty,
      setSuspended: (suspended) => this.update({ suspended }),
    })
  }
  subscribe = (listener: () => void) => {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }
  snapshot = () => this.state
  private update(patch: Partial<BitmapDraft>) {
    this.state = { ...this.state, ...patch }
    for (const listener of this.listeners) listener()
  }
  async load(): Promise<void> {
    if (this.gestureSource) {
      this.refreshAfterGesture = true
      return
    }
    if (
      this.state.dirty ||
      this.pending ||
      hasPendingDocumentDraft(this.target.doc)
    )
      return
    const request = ++this.request
    this.update({ busy: true, error: null })
    try {
      const result = await call('bitmap.glyph', this.target)
      if (request === this.request && !this.state.dirty)
        this.update({
          value: result.value ?? structuredClone(this.empty),
          resolved: result.resolved,
          generation: result.generation,
        })
    } catch (error) {
      if (request === this.request) this.update({ error })
      throw error
    } finally {
      if (request === this.request) this.update({ busy: false })
    }
  }
  edit(value: BitmapGlyph): void {
    if (this.state.busy || this.state.suspended) return
    ++this.request
    noteDocumentEdit(this.target.doc)
    this.update({ value, resolved: null, dirty: true, error: null })
  }
  /** Preserve earlier edits and errors separately from one cancellable drag. */
  beginGesture(accept?: () => void): void {
    if (!this.state.busy && !this.state.suspended) {
      this.gestureSource = this.state
      this.acceptGesture = accept
    }
  }
  /** A completed drag keeps its draft and may now be flushed as one edit. */
  endGesture(): void {
    this.gestureSource = undefined
    this.acceptGesture = undefined
    if (this.refreshAfterGesture) {
      this.refreshAfterGesture = false
      // load records failures in the draft; deferred refresh has no caller.
      void this.load().catch(() => undefined)
    }
  }
  /** Restore the pre-drag draft without writing or undoing any earlier edit. */
  cancelGesture(): void {
    const source = this.gestureSource
    if (!source || this.pending) return
    ++this.request
    this.update({
      value: source.value,
      resolved: source.resolved,
      dirty: source.dirty,
      error: source.error,
    })
    this.endGesture()
  }
  flush(): Promise<void> {
    if (this.pending) return this.pending
    // A save/navigation barrier accepts the current draft, ending cancellation.
    const accept = this.acceptGesture
    this.endGesture()
    accept?.()
    if (!this.state.dirty || !this.state.value) return Promise.resolve()
    const value = this.state.value
    this.update({ busy: true })
    const pending = call('bitmap.apply', {
      doc: this.target.doc,
      expected_generation: this.state.generation,
      label: 'Edit bitmap pixels',
      command: {
        SetGlyph: {
          ppem: this.target.ppem,
          depth: this.target.depth,
          glyph: this.target.glyph,
          value,
        },
      },
    })
      .then(async () => {
        this.update({ dirty: false })
        const result = await call('bitmap.glyph', this.target)
        this.update({
          value: result.value ?? structuredClone(this.empty),
          resolved: result.resolved,
          generation: result.generation,
          error: null,
        })
      })
      .catch((error: unknown) => {
        this.update({ error })
        throw error
      })
      .finally(() => {
        this.pending = undefined
        this.update({ busy: false })
      })
    this.pending = pending
    return pending
  }
  /** Explicitly discard a conflicted local draft; normal navigation never discards. */
  async discard(): Promise<void> {
    if (this.pending) await this.pending
    this.update({ dirty: false })
    await this.load()
  }
}
