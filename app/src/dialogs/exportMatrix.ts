import type { ExportOptions, ExportReport } from '../api'
import type { ExportProfile } from '../stores/exportProfileStore'

import { FORMAT_EXTENSIONS, SFNT_FORMATS } from './exportFormats'

export type ExportMatrixErrorCode =
  | 'empty-directory'
  | 'empty-profiles'
  | 'empty-instances'
  | 'too-many-items'
  | 'invalid-instance'
  | 'unsupported-format'
  | 'path-collision'
  | 'invalid-name'

/** A validation failure whose code can be translated by the dialog. */
export class ExportMatrixError extends Error {
  constructor(
    public readonly code: ExportMatrixErrorCode,
    message: string,
  ) {
    super(message)
    this.name = 'ExportMatrixError'
  }
}

export interface ExportMatrixItem {
  id: string
  profileName: string
  instanceName: string
  path: string
  options: ExportOptions
}

export interface ExportMatrixResult {
  item: ExportMatrixItem
  report: ExportReport | null
  error: unknown | null
}

export const MAX_EXPORT_MATRIX_ITEMS = 100

function safeName(name: string): string {
  const safe = name
    .normalize('NFC')
    .replace(/[^\p{L}\p{N}._-]+/gu, '-')
    .replace(/^[._-]+|[._-]+$/g, '')
  if (!safe) {
    throw new ExportMatrixError(
      'invalid-name',
      'An instance or profile name cannot form a filename.',
    )
  }
  return safe
}

function validatePaths(items: readonly ExportMatrixItem[]): void {
  if (items.length > MAX_EXPORT_MATRIX_ITEMS) {
    throw new ExportMatrixError(
      'too-many-items',
      `An export matrix can contain at most ${MAX_EXPORT_MATRIX_ITEMS} items.`,
    )
  }
  const paths = new Set<string>()
  for (const item of items) {
    const path = item.path
      .normalize('NFC')
      .replace(/[\\/]+/g, '/')
      .toLowerCase()
    if (!path.trim()) {
      throw new ExportMatrixError('invalid-name', 'An output path is empty.')
    }
    if (paths.has(path)) {
      throw new ExportMatrixError(
        'path-collision',
        `Output path collision: ${item.path}`,
      )
    }
    paths.add(path)
  }
}

/** Plan and validate the complete matrix before any file can be written. */
export function planExportMatrix(
  directory: string,
  profiles: readonly ExportProfile[],
  instances: readonly { index: number; name: string }[],
): ExportMatrixItem[] {
  const destination = directory.trim()
  if (!destination) {
    throw new ExportMatrixError(
      'empty-directory',
      'Choose an output directory.',
    )
  }
  if (profiles.length === 0) {
    throw new ExportMatrixError(
      'empty-profiles',
      'Select at least one profile.',
    )
  }
  if (instances.length === 0) {
    throw new ExportMatrixError(
      'empty-instances',
      'Select at least one instance.',
    )
  }
  if (profiles.length * instances.length > MAX_EXPORT_MATRIX_ITEMS) {
    throw new ExportMatrixError(
      'too-many-items',
      `An export matrix can contain at most ${MAX_EXPORT_MATRIX_ITEMS} items.`,
    )
  }
  const indexes = new Set<number>()
  for (const instance of instances) {
    if (
      !Number.isSafeInteger(instance.index) ||
      instance.index < 0 ||
      indexes.has(instance.index)
    ) {
      throw new ExportMatrixError(
        'invalid-instance',
        'Each instance index must be a unique nonnegative integer.',
      )
    }
    indexes.add(instance.index)
  }
  const separator =
    destination.includes('\\') && !destination.includes('/') ? '\\' : '/'
  const prefix = destination.replace(/[\\/]+$/, '') + separator
  const items = profiles.flatMap((profile, profileIndex) => {
    if (!SFNT_FORMATS.includes(profile.options.format)) {
      throw new ExportMatrixError(
        'unsupported-format',
        'The profile format does not support named instance exports.',
      )
    }
    return instances.map((instance) => ({
      id: `${profile.id}:${profileIndex}:${instance.index}`,
      profileName: profile.name,
      instanceName: instance.name,
      path: `${prefix}${safeName(instance.name)}-${safeName(profile.name)}${FORMAT_EXTENSIONS[profile.options.format]}`,
      options: {
        ...structuredClone(profile.options),
        instance: instance.index,
        variable: false,
        master_subset: undefined,
        master: null,
        keep_tt_instructions: false,
      },
    }))
  })
  validatePaths(items)
  return items
}

/** Export an immutable batch snapshot sequentially, retaining individual failures. */
export async function runExportMatrix(
  items: readonly ExportMatrixItem[],
  exportItem: (item: ExportMatrixItem) => Promise<ExportReport>,
  onResult?: (result: ExportMatrixResult) => void,
  shouldContinue: () => boolean = () => true,
): Promise<ExportMatrixResult[]> {
  const snapshot = structuredClone(items)
  validatePaths(snapshot)
  const results: ExportMatrixResult[] = []
  for (const item of snapshot) {
    if (!shouldContinue()) break
    let result: ExportMatrixResult
    try {
      result = { item, report: await exportItem(item), error: null }
    } catch (error) {
      result = { item, report: null, error }
    }
    results.push(result)
    onResult?.(result)
  }
  return results
}
