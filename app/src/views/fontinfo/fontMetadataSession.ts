import { call, type DocId, type FontInfo } from '../../api'
import { registerDocumentEditParticipant } from '../../editor/documentEditParticipants'
import { noteDocumentEdit } from '../../editor/editSession'
import { i18n } from '../../i18n'

/** Existing scalar metadata fields supported by the staged editor. */
export const FONT_METADATA_FIELDS = [
  'family_name',
  'style_name',
  'units_per_em',
  'version_major',
  'version_minor',
  'copyright',
  'trademark',
  'note',
  'open_type_name_designer',
  'open_type_name_designer_url',
  'open_type_name_manufacturer',
  'open_type_name_manufacturer_url',
  'open_type_name_license',
  'open_type_name_license_url',
  'open_type_name_version',
  'open_type_name_unique_id',
  'open_type_name_description',
  'open_type_name_preferred_family_name',
  'open_type_name_preferred_subfamily_name',
  'open_type_name_compatible_full_name',
  'open_type_name_sample_text',
  'open_type_name_wws_family_name',
  'open_type_name_wws_subfamily_name',
  'postscript_font_name',
  'postscript_full_name',
] as const satisfies readonly (keyof FontInfo)[]

/** A supported field; complex classification/name records are preserved. */
export type FontMetadataField = (typeof FONT_METADATA_FIELDS)[number]
/** Keep numerical input as text so invalid partial input can survive failures. */
export type FontMetadataDraft = Record<FontMetadataField, string>

interface State {
  source: FontInfo | null
  draft: FontMetadataDraft | null
  dirty: boolean
  busy: boolean
  suspended: boolean
  error: unknown
}

function draftFrom(info: FontInfo): FontMetadataDraft {
  return Object.fromEntries(
    FONT_METADATA_FIELDS.map((field) => [field, String(info[field] ?? '')]),
  ) as FontMetadataDraft
}

function changedFields(source: FontInfo, draft: FontMetadataDraft) {
  return FONT_METADATA_FIELDS.filter(
    (field) => draft[field] !== String(source[field] ?? ''),
  )
}

function validatedPatch(source: FontInfo, draft: FontMetadataDraft) {
  const patch: Partial<Pick<FontInfo, FontMetadataField>> = {}
  for (const field of changedFields(source, draft)) {
    const text = draft[field]
    if (
      field === 'units_per_em' ||
      field === 'version_major' ||
      field === 'version_minor'
    ) {
      const value = Number(text)
      const min = field === 'units_per_em' ? 1 : 0
      const max = field === 'units_per_em' ? 65535 : 4294967295
      if (
        !/^\d+$/.test(text) ||
        !Number.isSafeInteger(value) ||
        value < min ||
        value > max
      )
        throw new Error(i18n.t('fontMetadata.invalidNumber', { min, max }))
      patch[field] = value
    } else if (field === 'family_name' || field === 'style_name') {
      if (!text.trim()) throw new Error(i18n.t('fontMetadata.requiredNames'))
      patch[field] = text
    } else {
      patch[field] = text === '' ? null : text
    }
  }
  return patch
}

/** Staged scalar metadata; fresh untouched fields survive a whole-info command. */
export class FontMetadataSession {
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
  private request = 0

  constructor(
    readonly doc: DocId,
    readonly label: string,
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

  /** Participates in save, history and navigation without awaiting its own gate. */
  connect() {
    return registerDocumentEditParticipant(this.doc, {
      flush: () => this.flush(),
      hasPendingEdits: () => this.state.dirty,
      setSuspended: (suspended) => this.update({ suspended }),
    })
  }

  async load(): Promise<void> {
    if (this.state.dirty || this.pending) return
    const request = ++this.request
    this.update({ busy: true, error: null })
    try {
      const { info } = await call('doc.font_info', { doc: this.doc })
      if (request === this.request && !this.state.dirty)
        this.update({ source: info, draft: draftFrom(info) })
    } catch (error) {
      if (request === this.request) this.update({ error })
      throw error
    } finally {
      if (request === this.request) this.update({ busy: false })
    }
  }

  edit(patch: Partial<FontMetadataDraft>): void {
    const { source, draft } = this.state
    if (!source || !draft || this.state.busy || this.state.suspended) return
    const next = { ...draft, ...patch }
    ++this.request
    noteDocumentEdit(this.doc)
    this.update({
      draft: next,
      dirty: changedFields(source, next).length > 0,
      error: null,
    })
  }

  /** Closes the edit token even when reading, merging or applying fails. */
  private async apply(
    source: FontInfo,
    patch: Partial<FontInfo>,
  ): Promise<FontInfo> {
    const token = await call('doc.edit_begin', {
      doc: this.doc,
      label: this.label,
    })
    let next: FontInfo
    try {
      const { info } = await call('doc.font_info', { doc: this.doc })
      // FontInfoView has no generation. The transaction token protects the
      // read/apply interval; compare each touched field with its draft baseline.
      // An identical desired value also permits retry after a lost end response.
      const fields = Object.keys(patch) as FontMetadataField[]
      if (
        fields.some(
          (field) =>
            info[field] !== source[field] && info[field] !== patch[field],
        )
      )
        throw new Error(i18n.t('fontMetadata.conflict'))
      next = { ...info, ...patch }
      if (fields.some((field) => info[field] !== next[field]))
        await call('doc.edit_apply', {
          doc: this.doc,
          token,
          command: { SetFontInfo: { info: next } },
        })
    } catch (error) {
      await call('doc.edit_end', { doc: this.doc, token }).catch(
        () => undefined,
      )
      throw error
    }
    await call('doc.edit_end', { doc: this.doc, token })
    return next
  }

  flush(): Promise<void> {
    if (this.pending) return this.pending
    const { source, draft, dirty } = this.state
    if (!source || !draft || !dirty) return Promise.resolve()
    let patch: Partial<FontInfo>
    try {
      patch = validatedPatch(source, draft)
    } catch (error) {
      this.update({ error })
      return Promise.reject(error)
    }
    this.update({ busy: true, error: null })
    this.pending = this.apply(source, patch)
      .then((info) => {
        this.update({ source: info, draft: draftFrom(info), dirty: false })
      })
      .catch((error: unknown) => {
        this.update({ error })
        throw error
      })
      .finally(() => {
        this.pending = undefined
        this.update({ busy: false })
      })
    return this.pending
  }

  async discard(): Promise<void> {
    if (this.pending) await this.pending
    this.update({ dirty: false })
    await this.load()
  }
}
