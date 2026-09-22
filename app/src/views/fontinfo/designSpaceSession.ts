import {
  call,
  type DocId,
  type VariationState,
  type Axis,
  type Master,
} from '../../api'
import { registerDocumentEditParticipant } from '../../editor/documentEditParticipants'
import { noteDocumentEdit } from '../../editor/editSession'
import { i18n } from '../../i18n'

export type EditableAxis = Omit<Axis, 'tag'> & { tag: string }
export type DesignSpaceValue = Omit<VariationState, 'axes'> & {
  axes: EditableAxis[]
}
const editable = (value: VariationState): DesignSpaceValue => ({
  ...value,
  axes: value.axes.map((axis) => ({
    ...axis,
    tag: String.fromCharCode(...axis.tag),
  })),
})
interface Draft {
  value: DesignSpaceValue | null
  dirty: boolean
  busy: boolean
  suspended: boolean
  error: unknown
}

/** Revision-checked design-space draft; failed writes retain the user's values. */
export class DesignSpaceSession {
  private state: Draft = {
    value: null,
    dirty: false,
    busy: false,
    suspended: false,
    error: null,
  }
  private listeners = new Set<() => void>()
  private pending?: Promise<void>
  private request = 0
  private copyFrom = ''
  private originalTags = new Set<string>()
  private axisRenames: Record<string, string> = {}
  private sourceMaster: Master | undefined
  copySource(): Master | undefined {
    return this.sourceMaster ? structuredClone(this.sourceMaster) : undefined
  }
  constructor(
    readonly doc: DocId,
    readonly label: string,
  ) {}
  subscribe = (listener: () => void) => {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }
  snapshot = () => this.state
  private update(patch: Partial<Draft>) {
    this.state = { ...this.state, ...patch }
    for (const listener of this.listeners) listener()
  }
  connect(commitFields: () => void): () => void {
    return registerDocumentEditParticipant(this.doc, {
      flush: () => {
        commitFields()
        return this.flush()
      },
      hasPendingEdits: () => this.state.dirty,
      setSuspended: (suspended) => this.update({ suspended }),
    })
  }
  async load(): Promise<void> {
    if (this.state.dirty || this.pending) return
    const request = ++this.request
    this.update({ busy: true, error: null })
    try {
      const value = await call('variation.state', { doc: this.doc })
      if (request === this.request && !this.state.dirty) {
        this.originalTags = new Set(
          value.axes.map((axis) => String.fromCharCode(...axis.tag)),
        )
        this.axisRenames = {}
        this.copyFrom = value.masters[0]?.id ?? ''
        this.sourceMaster = value.masters[0]
        this.update({ value: editable(value) })
      }
    } catch (error) {
      if (request === this.request) this.update({ error })
      throw error
    } finally {
      if (request === this.request) this.update({ busy: false })
    }
  }
  renameAxis(old: string, next: string): void {
    if (this.state.busy || this.state.suspended || old === next) return
    const origin =
      Object.keys(this.axisRenames).find(
        (tag) => this.axisRenames[tag] === old,
      ) ?? (this.originalTags.has(old) ? old : undefined)
    if (origin) {
      if (origin === next) delete this.axisRenames[origin]
      else this.axisRenames[origin] = next
    }
  }
  edit(value: DesignSpaceValue): void {
    if (this.state.busy || this.state.suspended) return
    ++this.request
    noteDocumentEdit(this.doc)
    this.update({ value, dirty: true, error: null })
  }
  flush(): Promise<void> {
    if (this.pending) return this.pending
    const value = this.state.value
    if (!this.state.dirty || !value) return Promise.resolve()
    if (value.axes.some((axis) => !/^[ -~]{4}$/.test(axis.tag))) {
      const error = new Error(i18n.t('variation.invalidAxisTag'))
      this.update({ error })
      return Promise.reject(error)
    }
    this.update({ busy: true })
    const pending = call('variation.apply', {
      doc: this.doc,
      expected_generation: value.generation,
      label: this.label,
      command: {
        Set: {
          axis_renames: Object.fromEntries(
            Object.entries(this.axisRenames).filter(([, tag]) =>
              value.axes.some((axis) => axis.tag === tag),
            ),
          ),
          axes: value.axes.map((axis) => ({
            ...axis,
            tag: [...axis.tag].map((c) => c.charCodeAt(0)) as Axis['tag'],
          })),
          masters: value.masters,
          instances: value.instances,
          copy_from: this.copyFrom,
        },
      },
    })
      .then(async () => {
        this.update({ dirty: false, value: null })
        const value = await call('variation.state', { doc: this.doc })
        this.originalTags = new Set(
          value.axes.map((axis) => String.fromCharCode(...axis.tag)),
        )
        this.axisRenames = {}
        this.copyFrom = value.masters[0]?.id ?? ''
        this.sourceMaster = value.masters[0]
        this.update({ value: editable(value), error: null })
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
  async discard(): Promise<void> {
    if (this.pending) await this.pending
    this.update({ dirty: false })
    await this.load()
  }
}
