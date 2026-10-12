import { beforeEach, describe, expect, it } from 'vitest'
import {
  BITMAP_VIEW_STORAGE_KEY,
  bitmapEditorActive,
  readOverlays,
  useBitmapToolStore,
} from './bitmapToolStore'

beforeEach(() => {
  localStorage.clear()
  useBitmapToolStore.setState({
    tool: 'pencil',
    overlays: readOverlays(),
    editors: 0,
  })
})

describe('strike editor tool and overlay state', () => {
  it('starts with the pencil and the default overlays', () => {
    expect(useBitmapToolStore.getState().tool).toBe('pencil')
    expect(useBitmapToolStore.getState().overlays).toEqual({
      grid: true,
      outline: false,
      advanceColumn: true,
      fontUnits: true,
    })
  })

  it('persists each overlay independently and reads it back', () => {
    useBitmapToolStore.getState().toggleOverlay('outline')
    useBitmapToolStore.getState().setOverlay('grid', false)
    expect(readOverlays()).toEqual({
      grid: false,
      outline: true,
      advanceColumn: true,
      fontUnits: true,
    })
  })

  it('falls back to the defaults for missing, corrupt or wrongly typed storage', () => {
    localStorage.setItem(BITMAP_VIEW_STORAGE_KEY, 'not json')
    expect(readOverlays().grid).toBe(true)
    localStorage.setItem(BITMAP_VIEW_STORAGE_KEY, '[]')
    expect(readOverlays().advanceColumn).toBe(true)
    localStorage.setItem(
      BITMAP_VIEW_STORAGE_KEY,
      '{"grid":"yes","outline":true}',
    )
    expect(readOverlays()).toEqual({
      grid: true,
      outline: true,
      advanceColumn: true,
      fontUnits: true,
    })
  })

  it('counts mounted editors so tool shortcuts bind only while one is open', () => {
    expect(bitmapEditorActive()).toBe(false)
    const first = useBitmapToolStore.getState().registerEditor()
    const second = useBitmapToolStore.getState().registerEditor()
    expect(bitmapEditorActive()).toBe(true)
    first()
    expect(bitmapEditorActive()).toBe(true)
    second()
    expect(bitmapEditorActive()).toBe(false)
    // A stray extra release cannot push the count negative.
    second()
    expect(useBitmapToolStore.getState().editors).toBe(0)
  })
})
