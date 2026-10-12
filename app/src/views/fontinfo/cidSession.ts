import { call, type CidInfo, type CidState, type DocId } from '../../api'
import { registerDocumentEditParticipant } from '../../editor/documentEditParticipants'
import { noteDocumentEdit } from '../../editor/editSession'

export interface CidDraft {
  enabled: boolean
  registry: string
  ordering: string
  supplement: string
  mapping: string
  reorder: boolean
}
interface State {
  source: CidState | null
  draft: CidDraft | null
  dirty: boolean
  busy: boolean
  suspended: boolean
  error: unknown
}

/** Parse editable CID/name lines without silently overwriting duplicate numbers. */
export function parseCid(draft: CidDraft, invalid: string): CidInfo | null {
  if (!draft.enabled) return null
  const supplement = Number(draft.supplement)
  if (
    !/^\d+$/.test(draft.supplement) ||
    !Number.isSafeInteger(supplement) ||
    supplement > 0x7fffffff
  )
    throw new Error(invalid)
  const cid_map: Record<number, string> = {}
  const names = new Set<string>()
  if (draft.mapping.length > 4_000_000) throw new Error(invalid)
  for (const line of draft.mapping.split(/\r?\n/)) {
    if (!line.trim()) continue
    const match = /^\s*(\d+)\s+(\S+)\s*$/.exec(line)
    if (!match) throw new Error(invalid)
    const cid = Number(match[1]),
      name = match[2]!
    if (
      cid > 65535 ||
      cid in cid_map ||
      names.has(name) ||
      (cid === 0) !== (name === '.notdef')
    )
      throw new Error(invalid)
    cid_map[cid] = name
    names.add(name)
  }
  return {
    registry: draft.registry,
    ordering: draft.ordering,
    supplement,
    cid_map,
  }
}

/**
 * Exported glyph names the mapping does not cover, in glyph order.
 *
 * CID-keyed CFF export needs a CID for every *exported* glyph and refuses the
 * whole font otherwise, so the editor reports the gap while it can still be
 * fixed. Glyphs that do not export never reach the writer's glyph order and are
 * therefore never reported; `.notdef` is excluded too, because the writer
 * always gives it CID 0. An unparseable draft reports nothing, because the
 * invalid-entry notice already explains what is wrong.
 */
export function unmappedGlyphs(
  source: CidState,
  draft: CidDraft,
  invalid: string,
): readonly string[] {
  if (!draft.enabled) return []
  let mapped: Set<string>
  try {
    mapped = new Set(Object.values(parseCid(draft, invalid)?.cid_map ?? {}))
  } catch {
    return []
  }
  const exported = new Set(source.exported)
  return source.glyphs.filter(
    (name) => name !== '.notdef' && exported.has(name) && !mapped.has(name),
  )
}

/** Fill an Adobe-Identity mapping, retaining existing IDs and glyph order. */
export function identityMapping(
  source: CidState,
  draft: CidDraft,
  invalid: string,
): string {
  if (draft.registry !== 'Adobe' || draft.ordering !== 'Identity')
    throw new Error(invalid)
  const mapping = parseCid({ ...draft, enabled: true }, invalid)!.cid_map
  const names = new Set(Object.values(mapping))
  let next = 1
  for (const name of source.glyphs) {
    if (names.has(name)) continue
    if (name === '.notdef') mapping[0] = name
    else {
      while (next in mapping) next++
      if (next > 65535) throw new Error(invalid)
      mapping[next++] = name
    }
  }
  return Object.entries(mapping)
    .map(([cid, name]) => `${cid}\t${name}`)
    .join('\n')
}

/** CID drafts join document save, navigation and undo handovers. */
export class CidSession {
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
    readonly label: string,
    readonly invalid: string,
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
  connect() {
    return registerDocumentEditParticipant(this.doc, {
      flush: () => this.flush(),
      hasPendingEdits: () => this.state.dirty,
      setSuspended: (suspended) => this.update({ suspended }),
    })
  }
  async load() {
    if (this.state.dirty || this.pending) return
    this.update({ busy: true })
    try {
      const source = await call('cid.state', { doc: this.doc }),
        cid = source.cid
      this.update({
        source,
        draft: {
          enabled: cid !== null,
          registry: cid?.registry ?? 'Adobe',
          ordering: cid?.ordering ?? 'Identity',
          supplement: String(cid?.supplement ?? 0),
          mapping: Object.entries(cid?.cid_map ?? {})
            .map(([id, name]) => `${id}\t${name}`)
            .join('\n'),
          reorder: false,
        },
        error: null,
      })
    } catch (error) {
      this.update({ error })
      throw error
    } finally {
      this.update({ busy: false })
    }
  }
  edit(patch: Partial<CidDraft>) {
    if (!this.state.draft || this.state.busy || this.state.suspended) return
    noteDocumentEdit(this.doc)
    this.update({
      draft: { ...this.state.draft, ...patch },
      dirty: true,
      error: null,
    })
  }
  flush(): Promise<void> {
    if (this.pending) return this.pending
    const { source, draft } = this.state
    if (!source || !draft || !this.state.dirty) return Promise.resolve()
    let cid: CidInfo | null
    try {
      cid = parseCid(draft, this.invalid)
    } catch (error) {
      this.update({ error })
      return Promise.reject(error)
    }
    this.update({ busy: true })
    this.pending = call('cid.apply', {
      doc: this.doc,
      expected_generation: source.generation,
      cid,
      reorder: draft.enabled && draft.reorder,
      label: this.label,
    })
      .then(() => {
        this.update({ dirty: false, source: null })
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
  async discard() {
    if (this.pending) await this.pending
    this.update({ dirty: false })
    await this.load()
  }
}
