import { call, type DocId, type MathState, type MathTable } from '../../api'
import { registerDocumentEditParticipant } from '../../editor/documentEditParticipants'
import { noteDocumentEdit } from '../../editor/editSession'
import { diffCommands } from './mathModel'

interface State {
  source: MathState | null
  /** Editable draft; null means no table (absent or deleted). */
  draft: MathTable | null
  dirty: boolean
  busy: boolean
  suspended: boolean
  error: unknown
}

/** MATH drafts join document save, navigation and undo handovers. */
export class MathSession {
  private state: State = {
    source: null,
    draft: null,
    dirty: false,
    busy: false,
    suspended: false,
    error: null,
  }
  private listeners = new Set<() => void>()
  private pending?: Promise<void>
  constructor(
    readonly doc: DocId,
    readonly labels: { edit: string; decode: string },
  ) {}
  snapshot = () => this.state
  subscribe = (listener: () => void) => {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }
  private update(patch: Partial<State>) {
    this.state = { ...this.state, ...patch }
    for (const listener of this.listeners) listener()
  }
  connect(validate: () => void = () => {}) {
    return registerDocumentEditParticipant(this.doc, {
      flush: async () => {
        // Unparsed text still lives in the form, outside the semantic draft.
        // Save/navigation must reject it just as the Apply button does.
        validate()
        await this.flush()
      },
      hasPendingEdits: () => this.state.dirty,
      setSuspended: (suspended) => this.update({ suspended }),
    })
  }
  /** Reload from the document unless a draft or an apply is outstanding. */
  async load() {
    if (this.state.dirty || this.pending) return
    this.update({ busy: true })
    try {
      const source = await call('math.state', { doc: this.doc })
      this.update({ source, draft: source.math, error: null })
    } catch (error) {
      this.update({ error })
      throw error
    } finally {
      this.update({ busy: false })
    }
  }
  /** Replace the draft; the caller passes a new object for every change. */
  edit(draft: MathTable | null) {
    if (!this.state.source || this.state.busy || this.state.suspended) return
    noteDocumentEdit(this.doc)
    this.update({ draft, dirty: true, error: null })
  }
  private run(request: () => Promise<unknown>): Promise<void> {
    this.update({ busy: true })
    this.pending = request()
      .then(() => {
        // Keep the stale source until `load` replaces it so the form stays
        // mounted and the selected record survives an apply.
        this.update({ dirty: false })
      })
      .catch((error: unknown) => {
        this.update({ error })
        throw error
      })
      .finally(() => {
        this.pending = undefined
        this.update({ busy: false })
      })
    return this.pending.then(() => this.load())
  }
  flush(): Promise<void> {
    if (this.pending) return this.pending
    const { source, draft, dirty } = this.state
    if (!source || !dirty) return Promise.resolve()
    const commands = diffCommands(source, draft)
    if (commands.length === 0) {
      this.update({ dirty: false })
      return Promise.resolve()
    }
    return this.run(() =>
      call('math.apply', {
        doc: this.doc,
        expected_generation: source.generation,
        commands,
        label: this.labels.edit,
      }),
    )
  }
  /** Convert imported table bytes into editable data as one undo step. */
  decode(): Promise<void> {
    const { source, dirty } = this.state
    if (this.pending || !source || dirty) return Promise.resolve()
    return this.run(() =>
      call('math.decode_imported', {
        doc: this.doc,
        expected_generation: source.generation,
        label: this.labels.decode,
      }),
    )
  }
  async discard() {
    if (this.pending) await this.pending
    this.update({ dirty: false })
    await this.load()
  }
}
