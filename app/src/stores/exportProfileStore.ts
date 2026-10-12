import { create } from 'zustand'

import {
  call,
  type DocId,
  type ExportProfile,
  type ExportProfilesResult,
  type VariableExportConfig,
} from '../api'

import { onDocumentForgotten } from './documentStore'

const MAX_NAME_LENGTH = 80
const MAX_PATH_LENGTH = 200

/** Bound the profile picker and the profile/instance export matrix. */
export const MAX_EXPORT_PROFILES = 24

/** Bound the stored variable-font configurations. */
export const MAX_VARIABLE_CONFIGS = 24

/** Stable error codes translated by the profile picker. */
export type ExportProfileErrorCode =
  | 'blankName'
  | 'nameTooLong'
  | 'duplicateName'
  | 'limitReached'
  | 'invalidSubfolder'
  | 'invalidFileName'
  | 'notFound'
  | 'kernelRejected'

/** A validation or transport failure; callers must not report a saved profile. */
export class ExportProfileError extends Error {
  constructor(
    readonly code: ExportProfileErrorCode,
    readonly detail: string | null = null,
  ) {
    super(code)
    this.name = 'ExportProfileError'
  }
}

/** The export configurations one open document stores in its project file. */
export interface StoredExportConfigurations {
  profiles: ExportProfile[]
  variableConfigs: VariableExportConfig[]
}

const EMPTY: StoredExportConfigurations = { profiles: [], variableConfigs: [] }

function nameKey(name: string): string {
  return name.trim().normalize('NFC').toLowerCase()
}

function checkName(
  name: string,
  existing: readonly { name: string }[],
  replacing: string | null,
  limit: number,
): string {
  const trimmed = name.trim()
  if (!trimmed) throw new ExportProfileError('blankName')
  if (trimmed.length > MAX_NAME_LENGTH)
    throw new ExportProfileError('nameTooLong')
  const clash = existing.some(
    (entry) =>
      nameKey(entry.name) === nameKey(trimmed) &&
      (replacing === null || nameKey(entry.name) !== nameKey(replacing)),
  )
  if (clash) throw new ExportProfileError('duplicateName')
  const replaces =
    replacing !== null &&
    existing.some((entry) => nameKey(entry.name) === nameKey(replacing))
  if (!replaces && existing.length >= limit)
    throw new ExportProfileError('limitReached')
  return trimmed
}

/** Reject anything that could escape the chosen destination folder. */
export function checkSubfolder(subfolder: string): string {
  const trimmed = subfolder.trim().replace(/\\/g, '/').replace(/\/+$/, '')
  if (!trimmed) return ''
  if (
    trimmed.length > MAX_PATH_LENGTH ||
    trimmed.startsWith('/') ||
    trimmed.includes(':') ||
    // eslint-disable-next-line no-control-regex
    /[\u0000-\u001f]/.test(trimmed) ||
    trimmed
      .split('/')
      .some((part) => part === '' || part === '.' || part === '..')
  )
    throw new ExportProfileError('invalidSubfolder')
  return trimmed
}

/** Reject a file name that carries a path or a leading dot. */
export function checkFileName(fileName: string): string {
  const trimmed = fileName.trim()
  if (!trimmed) return ''
  if (
    trimmed.length > MAX_PATH_LENGTH ||
    /[/\\:]/.test(trimmed) ||
    // eslint-disable-next-line no-control-regex
    /[\u0000-\u001f]/.test(trimmed) ||
    trimmed.startsWith('.')
  )
    throw new ExportProfileError('invalidFileName')
  return trimmed
}

/** Project-stored export configurations, shared by the dialog, matrix and CLI. */
export interface ExportProfileState {
  docs: Record<number, StoredExportConfigurations>
  loading: Record<number, boolean>
  error: ExportProfileErrorCode | null
  detail: string | null
  stored: (doc: DocId) => StoredExportConfigurations
  load: (doc: DocId) => Promise<StoredExportConfigurations>
  saveProfile: (
    doc: DocId,
    profile: ExportProfile,
    replacing?: string | null,
  ) => Promise<ExportProfile>
  removeProfile: (doc: DocId, name: string) => Promise<void>
  saveConfig: (
    doc: DocId,
    config: VariableExportConfig,
    replacing?: string | null,
  ) => Promise<VariableExportConfig>
  removeConfig: (doc: DocId, name: string) => Promise<void>
  forget: (doc: DocId) => void
}

async function read(doc: DocId): Promise<StoredExportConfigurations> {
  const result: ExportProfilesResult = await call('export.profiles_get', {
    doc,
  })
  return {
    profiles: result.profiles,
    variableConfigs: result.variable_configs,
  }
}

async function write(
  doc: DocId,
  next: StoredExportConfigurations,
): Promise<StoredExportConfigurations> {
  try {
    const result: ExportProfilesResult = await call('export.profiles_set', {
      doc,
      profiles: next.profiles,
      variable_configs: next.variableConfigs,
    })
    return {
      profiles: result.profiles,
      variableConfigs: result.variable_configs,
    }
  } catch (cause) {
    throw new ExportProfileError(
      'kernelRejected',
      cause instanceof Error ? cause.message : String(cause),
    )
  }
}

export const useExportProfileStore = create<ExportProfileState>((set, get) => ({
  docs: {},
  loading: {},
  error: null,
  detail: null,
  stored: (doc) => get().docs[doc] ?? EMPTY,
  load: async (doc) => {
    set((state) => ({ loading: { ...state.loading, [doc]: true } }))
    try {
      const stored = await read(doc)
      set((state) => ({
        docs: { ...state.docs, [doc]: stored },
        loading: { ...state.loading, [doc]: false },
        error: null,
        detail: null,
      }))
      return stored
    } catch (cause) {
      set((state) => ({
        loading: { ...state.loading, [doc]: false },
        error: 'kernelRejected',
        detail: cause instanceof Error ? cause.message : String(cause),
      }))
      throw new ExportProfileError(
        'kernelRejected',
        cause instanceof Error ? cause.message : String(cause),
      )
    }
  },
  saveProfile: async (doc, profile, replacing = null) => {
    const current = await get().load(doc)
    const name = checkName(
      profile.name,
      current.profiles,
      replacing,
      MAX_EXPORT_PROFILES,
    )
    const entry: ExportProfile = {
      name,
      subfolder: checkSubfolder(profile.subfolder),
      file_name: checkFileName(profile.file_name),
      options: structuredClone(profile.options),
    }
    const profiles = current.profiles.some(
      (existing) => nameKey(existing.name) === nameKey(replacing ?? name),
    )
      ? current.profiles.map((existing) =>
          nameKey(existing.name) === nameKey(replacing ?? name)
            ? entry
            : existing,
        )
      : [...current.profiles, entry]
    const stored = await write(doc, { ...current, profiles })
    set((state) => ({ docs: { ...state.docs, [doc]: stored }, error: null }))
    return (
      stored.profiles.find((saved) => nameKey(saved.name) === nameKey(name)) ??
      entry
    )
  },
  removeProfile: async (doc, name) => {
    const current = await get().load(doc)
    const profiles = current.profiles.filter(
      (profile) => nameKey(profile.name) !== nameKey(name),
    )
    if (profiles.length === current.profiles.length)
      throw new ExportProfileError('notFound')
    const stored = await write(doc, { ...current, profiles })
    set((state) => ({ docs: { ...state.docs, [doc]: stored }, error: null }))
  },
  saveConfig: async (doc, config, replacing = null) => {
    const current = await get().load(doc)
    const name = checkName(
      config.name,
      current.variableConfigs,
      replacing,
      MAX_VARIABLE_CONFIGS,
    )
    const entry: VariableExportConfig = {
      name,
      file_name: checkFileName(config.file_name),
      masters: [...config.masters],
      axes: [...config.axes],
      instances: [...config.instances],
    }
    const variableConfigs = current.variableConfigs.some(
      (existing) => nameKey(existing.name) === nameKey(replacing ?? name),
    )
      ? current.variableConfigs.map((existing) =>
          nameKey(existing.name) === nameKey(replacing ?? name)
            ? entry
            : existing,
        )
      : [...current.variableConfigs, entry]
    const stored = await write(doc, { ...current, variableConfigs })
    set((state) => ({ docs: { ...state.docs, [doc]: stored }, error: null }))
    return (
      stored.variableConfigs.find(
        (saved) => nameKey(saved.name) === nameKey(name),
      ) ?? entry
    )
  },
  removeConfig: async (doc, name) => {
    const current = await get().load(doc)
    const variableConfigs = current.variableConfigs.filter(
      (config) => nameKey(config.name) !== nameKey(name),
    )
    if (variableConfigs.length === current.variableConfigs.length)
      throw new ExportProfileError('notFound')
    const stored = await write(doc, { ...current, variableConfigs })
    set((state) => ({ docs: { ...state.docs, [doc]: stored }, error: null }))
  },
  forget: (doc) =>
    set((state) => {
      if (!(doc in state.docs) && !(doc in state.loading)) return state
      const docs = { ...state.docs }
      const loading = { ...state.loading }
      delete docs[doc]
      delete loading[doc]
      return { docs, loading }
    }),
}))

onDocumentForgotten((doc) => useExportProfileStore.getState().forget(doc))
