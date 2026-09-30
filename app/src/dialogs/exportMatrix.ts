import type { ExportOptions, ExportReport } from '../api'

export type ExportMatrixErrorCode =
  | 'empty-directory'
  | 'empty-profiles'
  | 'empty-rows'
  | 'too-many-items'
  | 'unsafe-path'
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

/** What one matrix row exports: a named static instance or a variable config. */
export type ExportMatrixRowKind = 'instance' | 'variable'

/**
 * One planned output, already resolved by the kernel.
 *
 * `options` and `relativePath` come from `export.resolve_profile`, so the
 * dialog, the matrix and `fontcarver build --profile` derive destinations and
 * options from one implementation rather than from two that can drift.
 */
export interface ExportMatrixCell {
  profileName: string
  rowKind: ExportMatrixRowKind
  rowName: string
  options: ExportOptions
  relativePath: string
}

export interface ExportMatrixItem {
  id: string
  profileName: string
  rowKind: ExportMatrixRowKind
  rowName: string
  path: string
  options: ExportOptions
}

export interface ExportMatrixResult {
  item: ExportMatrixItem
  report: ExportReport | null
  error: unknown | null
}

export const MAX_EXPORT_MATRIX_ITEMS = 100

function checkRelative(relative: string): string {
  const normalized = relative.replace(/\\/g, '/').trim()
  if (!normalized) {
    throw new ExportMatrixError(
      'invalid-name',
      'A resolved file name is empty.',
    )
  }
  if (
    normalized.startsWith('/') ||
    normalized.includes(':') ||
    normalized
      .split('/')
      .some((part) => part === '' || part === '.' || part === '..')
  ) {
    throw new ExportMatrixError(
      'unsafe-path',
      `A resolved output path escapes the destination: ${relative}`,
    )
  }
  return normalized
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
  cells: readonly ExportMatrixCell[],
): ExportMatrixItem[] {
  const destination = directory.trim()
  if (!destination) {
    throw new ExportMatrixError(
      'empty-directory',
      'Choose an output directory.',
    )
  }
  if (cells.length === 0) {
    throw new ExportMatrixError(
      'empty-rows',
      'Select at least one instance or variable configuration and one profile.',
    )
  }
  if (cells.length > MAX_EXPORT_MATRIX_ITEMS) {
    throw new ExportMatrixError(
      'too-many-items',
      `An export matrix can contain at most ${MAX_EXPORT_MATRIX_ITEMS} items.`,
    )
  }
  const separator =
    destination.includes('\\') && !destination.includes('/') ? '\\' : '/'
  const prefix = destination.replace(/[\\/]+$/, '') + separator
  const items = cells.map((cell) => {
    const relative = checkRelative(cell.relativePath)
    return {
      id: `${cell.rowKind}:${cell.rowName}:${cell.profileName}`,
      profileName: cell.profileName,
      rowKind: cell.rowKind,
      rowName: cell.rowName,
      path: `${prefix}${separator === '\\' ? relative.replace(/\//g, '\\') : relative}`,
      options: structuredClone(cell.options),
    }
  })
  const ids = new Set<string>()
  for (const item of items) {
    if (ids.has(item.id)) {
      throw new ExportMatrixError(
        'invalid-name',
        `Two matrix cells share the identity ${item.id}.`,
      )
    }
    ids.add(item.id)
  }
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
