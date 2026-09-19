import { getTransport } from '../api'

/** Native document and font formats supported by the shared open handler. */
export const OPEN_FONT_EXTENSIONS = [
  'fcf',
  'fcf.gz',
  'fwf',
  'fwf.gz',
  'ttf',
  'otf',
  'ttc',
  'otc',
  'woff',
  'woff2',
  'ufo',
  'designspace',
  'glyphs',
  'sfd',
  'pfa',
  'pfb',
] as const

/** What a native file dialog should ask the user for. */
export interface PathRequest {
  readonly kind: 'open' | 'save'
  /** Translated display name of the file-type filter. */
  readonly filterName?: string
  /** Extensions the filter accepts, without the leading dot. */
  readonly extensions?: readonly string[]
  /** Path a save dialog starts from. */
  readonly defaultPath?: string
}

/** The outcome of asking the platform for a path. */
export type PathChoice =
  | { readonly kind: 'chosen'; readonly path: string }
  | { readonly kind: 'cancelled' }
  | { readonly kind: 'unsupported' }

/**
 * Whether this platform has a native file dialog.
 *
 * Only the Tauri shell does; under the browser/HTTP transport the interface
 * falls back to an in-app prompt, which is why callers ask before offering a
 * "Choose…" button at all.
 */
export function hasNativeFileDialog(): boolean {
  return getTransport().kind === 'tauri'
}

/**
 * Asks the platform for a path.
 *
 * This is the single place the application talks to the native file dialog:
 * everything else — opening a font, Save As, choosing an export destination —
 * goes through here and handles `unsupported` with its own in-app fallback (the
 * P0 `PathPromptDialog` for opening and saving, the destination field for the
 * export dialog).
 */
export async function choosePath(request: PathRequest): Promise<PathChoice> {
  if (!hasNativeFileDialog()) return { kind: 'unsupported' }
  const filters =
    request.filterName && request.extensions
      ? [{ name: request.filterName, extensions: [...request.extensions] }]
      : undefined
  const dialog = await import('@tauri-apps/plugin-dialog')
  const chosen =
    request.kind === 'open'
      ? await dialog.open({ multiple: false, directory: false, filters })
      : await dialog.save({ defaultPath: request.defaultPath, filters })
  return typeof chosen === 'string'
    ? { kind: 'chosen', path: chosen }
    : { kind: 'cancelled' }
}
