import { create } from 'zustand'
import type { PixelTool } from './pixels'

/** Storage shared by reloads and windows of this app origin. */
export const BITMAP_VIEW_STORAGE_KEY = 'fontcarver.bitmapView.v1'

/** Canvas overlays the strike editor remembers between sessions. */
export interface BitmapOverlays {
  /** Pixel grid lines. */
  grid: boolean
  /** Outline overlay of the master at the strike scale. */
  outline: boolean
  /** Shaded column from the origin to the horizontal advance. */
  advanceColumn: boolean
  /** Pointer and drag readouts in font units beside the pixel values. */
  fontUnits: boolean
}

const DEFAULT_OVERLAYS: BitmapOverlays = {
  grid: true,
  outline: false,
  advanceColumn: true,
  fontUnits: true,
}

/** Corrupt or absent storage never hides the editor's default overlays. */
export function readOverlays(): BitmapOverlays {
  try {
    const text = globalThis.localStorage?.getItem(BITMAP_VIEW_STORAGE_KEY)
    if (!text) return { ...DEFAULT_OVERLAYS }
    const stored: unknown = JSON.parse(text)
    if (!stored || typeof stored !== 'object') return { ...DEFAULT_OVERLAYS }
    const record = stored as Record<string, unknown>
    const pick = (key: keyof BitmapOverlays) =>
      typeof record[key] === 'boolean'
        ? (record[key] as boolean)
        : DEFAULT_OVERLAYS[key]
    return {
      grid: pick('grid'),
      outline: pick('outline'),
      advanceColumn: pick('advanceColumn'),
      fontUnits: pick('fontUnits'),
    }
  } catch {
    return { ...DEFAULT_OVERLAYS }
  }
}

function writeOverlays(overlays: BitmapOverlays): void {
  try {
    globalThis.localStorage?.setItem(
      BITMAP_VIEW_STORAGE_KEY,
      JSON.stringify(overlays),
    )
  } catch {
    /* A private window keeps the choice for this session only. */
  }
}

/**
 * The strike editor's own tool and overlay state.
 *
 * It lives in a store rather than in `BitmapEditor` so the Bitmap menu, the
 * command palette and the keyboard map can reach the same tool the toolbar
 * shows, and so the overlay switches survive a reload. `editors` counts mounted
 * strike editors, which is what gates the single-key tool shortcuts: with no
 * editor on screen those keys are not bound at all and keep their native
 * meaning elsewhere in the shell.
 */
export interface BitmapToolState {
  tool: PixelTool
  overlays: BitmapOverlays
  editors: number
  setTool: (tool: PixelTool) => void
  toggleOverlay: (key: keyof BitmapOverlays) => void
  setOverlay: (key: keyof BitmapOverlays, value: boolean) => void
  registerEditor: () => () => void
}

export const useBitmapToolStore = create<BitmapToolState>((set, get) => ({
  tool: 'pencil',
  overlays: readOverlays(),
  editors: 0,
  setTool: (tool) => set({ tool }),
  setOverlay: (key, value) => {
    const overlays = { ...get().overlays, [key]: value }
    writeOverlays(overlays)
    set({ overlays })
  },
  toggleOverlay: (key) => get().setOverlay(key, !get().overlays[key]),
  registerEditor: () => {
    set((state) => ({ editors: state.editors + 1 }))
    return () => set((state) => ({ editors: Math.max(0, state.editors - 1) }))
  },
}))

/** Whether a strike editor is on screen and can take a tool command. */
export function bitmapEditorActive(): boolean {
  return useBitmapToolStore.getState().editors > 0
}
