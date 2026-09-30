import { create } from 'zustand'

import type { ExportFormat, ExportOptions } from '../api'
import { FORMAT_EXTENSIONS } from '../dialogs/exportFormats'

const STORAGE_KEY = 'fontcarver.exportProfiles'
const STORAGE_VERSION = 1
const MAX_NAME_LENGTH = 80

/** Bound the local profile picker and the profile/instance export matrix. */
export const MAX_EXPORT_PROFILES = 24

/** A named snapshot of export settings, without a document or destination. */
export interface ExportProfile {
  id: string
  name: string
  options: ExportOptions
}

/** Stable error codes translated by the profile picker. */
export type ExportProfileErrorCode =
  | 'blankName'
  | 'nameTooLong'
  | 'duplicateName'
  | 'limitReached'
  | 'invalidOptions'
  | 'invalidStorage'
  | 'storageUnavailable'

/** A validation or durability failure; callers must not report a saved profile. */
export class ExportProfileError extends Error {
  constructor(readonly code: ExportProfileErrorCode) {
    super(code)
    this.name = 'ExportProfileError'
  }
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function text(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0
}

function positiveInteger(value: unknown, maximum: number): value is number {
  return (
    typeof value === 'number' &&
    Number.isInteger(value) &&
    value > 0 &&
    value <= maximum
  )
}

function flag(
  value: Record<string, unknown>,
  key: string,
  fallback?: boolean,
): boolean {
  const field = value[key] === undefined ? fallback : value[key]
  if (typeof field !== 'boolean') throw new ExportProfileError('invalidOptions')
  return field
}

function optionsSnapshot(value: unknown): ExportOptions {
  if (!record(value)) throw new ExportProfileError('invalidOptions')
  const {
    format,
    master,
    instance,
    master_subset,
    bitmap_strike,
    table_subset,
    glyph,
  } = value
  if (
    typeof format !== 'string' ||
    !Object.hasOwn(FORMAT_EXTENSIONS, format) ||
    !(master === null || text(master)) ||
    !(
      instance === undefined ||
      (typeof instance === 'number' &&
        Number.isSafeInteger(instance) &&
        instance >= 0)
    ) ||
    !(
      master_subset === undefined ||
      (Array.isArray(master_subset) &&
        master_subset.length <= 1024 &&
        master_subset.every(text))
    ) ||
    !(
      bitmap_strike === undefined ||
      (Array.isArray(bitmap_strike) &&
        bitmap_strike.length === 2 &&
        positiveInteger(bitmap_strike[0], 65535) &&
        positiveInteger(bitmap_strike[1], 32))
    ) ||
    !(
      table_subset === null ||
      (Array.isArray(table_subset) &&
        table_subset.length <= 4096 &&
        table_subset.every(
          (tag: unknown) =>
            typeof tag === 'string' && /^[\x20-\x7e]{4}$/.test(tag),
        ))
    ) ||
    !(glyph === undefined || text(glyph))
  )
    throw new ExportProfileError('invalidOptions')
  const options = {
    format: format as ExportFormat,
    master,
    variable: flag(value, 'variable'),
    remove_overlap: flag(value, 'remove_overlap'),
    autohint: flag(value, 'autohint'),
    keep_tt_instructions: flag(value, 'keep_tt_instructions'),
    preserve_untouched_tables: flag(value, 'preserve_untouched_tables'),
    production_names: flag(value, 'production_names'),
    add_extrema: flag(value, 'add_extrema', false),
    round_coordinates: flag(value, 'round_coordinates', false),
    decompose_components: flag(value, 'decompose_components', false),
    subroutinize: flag(value, 'subroutinize', false),
    table_subset:
      table_subset === null ? null : ([...table_subset] as string[]),
    ...(instance === undefined ? {} : { instance }),
    ...(master_subset === undefined
      ? {}
      : { master_subset: [...master_subset] as string[] }),
    ...(bitmap_strike === undefined
      ? {}
      : { bitmap_strike: [...bitmap_strike] as [number, number] }),
    ...(glyph === undefined ? {} : { glyph }),
  }
  return options
}

function nameKey(name: string): string {
  return name.trim().normalize('NFC').toLowerCase()
}

function readProfiles(): Pick<ExportProfileState, 'profiles' | 'error'> {
  let raw: string | null
  try {
    if (!globalThis.localStorage) throw new Error('Storage unavailable')
    raw = globalThis.localStorage.getItem(STORAGE_KEY)
  } catch {
    return { profiles: [], error: 'storageUnavailable' }
  }
  if (raw === null) return { profiles: [], error: null }
  try {
    const parsed: unknown = JSON.parse(raw)
    if (
      !record(parsed) ||
      parsed.version !== STORAGE_VERSION ||
      !Array.isArray(parsed.profiles) ||
      parsed.profiles.length > MAX_EXPORT_PROFILES
    )
      throw new ExportProfileError('invalidStorage')
    const ids = new Set<string>()
    const names = new Set<string>()
    const profiles = parsed.profiles.map((entry: unknown): ExportProfile => {
      if (
        !record(entry) ||
        !text(entry.id) ||
        !text(entry.name) ||
        entry.name.trim().length > MAX_NAME_LENGTH ||
        ids.has(entry.id) ||
        names.has(nameKey(entry.name))
      )
        throw new ExportProfileError('invalidStorage')
      ids.add(entry.id)
      names.add(nameKey(entry.name))
      return {
        id: entry.id,
        name: entry.name.trim(),
        options: optionsSnapshot(entry.options),
      }
    })
    return { profiles, error: null }
  } catch {
    return { profiles: [], error: 'invalidStorage' }
  }
}

function writeProfiles(profiles: ExportProfile[]): void {
  try {
    if (!globalThis.localStorage) throw new Error('Storage unavailable')
    globalThis.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ version: STORAGE_VERSION, profiles }),
    )
  } catch {
    throw new ExportProfileError('storageUnavailable')
  }
}

/** Durable, validated export presets shared by the picker and batch exporter. */
export interface ExportProfileState {
  profiles: ExportProfile[]
  error: ExportProfileErrorCode | null
  save: (name: string, options: ExportOptions) => ExportProfile
  remove: (id: string) => void
  reload: () => void
}

export const useExportProfileStore = create<ExportProfileState>((set, get) => ({
  ...readProfiles(),
  save: (name, options) => {
    const trimmed = name.trim()
    if (!trimmed) throw new ExportProfileError('blankName')
    if (trimmed.length > MAX_NAME_LENGTH)
      throw new ExportProfileError('nameTooLong')
    if (
      get().profiles.some(
        (profile) => nameKey(profile.name) === nameKey(trimmed),
      )
    )
      throw new ExportProfileError('duplicateName')
    if (get().profiles.length >= MAX_EXPORT_PROFILES)
      throw new ExportProfileError('limitReached')
    const profile = {
      id: crypto.randomUUID(),
      name: trimmed,
      options: optionsSnapshot(options),
    }
    const profiles = [...get().profiles, profile]
    writeProfiles(profiles)
    set({ profiles, error: null })
    return { ...profile, options: optionsSnapshot(profile.options) }
  },
  remove: (id) => {
    const profiles = get().profiles.filter((profile) => profile.id !== id)
    if (profiles.length === get().profiles.length) return
    writeProfiles(profiles)
    set({ profiles, error: null })
  },
  reload: () => set(readProfiles()),
}))
