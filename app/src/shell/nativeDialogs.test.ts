import { afterEach, describe, expect, it, vi } from 'vitest'

import { setTransport } from '../api'
import type { Transport } from '../transport'
import { useSessionStore } from '../stores/sessionStore'

import { choosePath, hasNativeFileDialog } from './nativeDialogs'

const open = vi.fn()
const save = vi.fn()

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: (...args: unknown[]) => open(...args) as unknown,
  save: (...args: unknown[]) => save(...args) as unknown,
}))

function transportOfKind(kind: Transport['kind']): Transport {
  return {
    kind,
    call: () => Promise.reject(new Error('not used')),
    onEvent: () => () => undefined,
  }
}

afterEach(() => {
  setTransport(undefined)
  open.mockReset()
  save.mockReset()
})

describe('choosePath', () => {
  it('reports that a transport without a native dialog cannot choose', async () => {
    setTransport(transportOfKind('http'))

    expect(hasNativeFileDialog()).toBe(false)
    expect(await choosePath({ kind: 'open' })).toEqual({ kind: 'unsupported' })
    expect(open).not.toHaveBeenCalled()
  })

  it('opens the native picker and returns the chosen path', async () => {
    setTransport(transportOfKind('tauri'))
    open.mockResolvedValue('/fonts/DejaVuSans.ttf')

    expect(hasNativeFileDialog()).toBe(true)
    expect(await choosePath({ kind: 'open' })).toEqual({
      kind: 'chosen',
      path: '/fonts/DejaVuSans.ttf',
    })
    expect(open).toHaveBeenCalledWith({
      multiple: false,
      directory: false,
      filters: undefined,
    })
  })

  it('passes the filter and the starting path to a save dialog', async () => {
    setTransport(transportOfKind('tauri'))
    save.mockResolvedValue('/out/DejaVuSans.ttf')

    const choice = await choosePath({
      kind: 'save',
      filterName: 'Font',
      extensions: ['ttf'],
      defaultPath: '/fonts/DejaVuSans.ttf',
    })

    expect(choice).toEqual({ kind: 'chosen', path: '/out/DejaVuSans.ttf' })
    expect(save).toHaveBeenCalledWith({
      defaultPath: '/fonts/DejaVuSans.ttf',
      filters: [{ name: 'Font', extensions: ['ttf'] }],
    })
  })

  it('reports a cancelled dialog', async () => {
    setTransport(transportOfKind('tauri'))
    save.mockResolvedValue(null)

    expect(await choosePath({ kind: 'save' })).toEqual({ kind: 'cancelled' })
  })

  it('offers native source formats in the Open Font filter', async () => {
    setTransport(transportOfKind('tauri'))
    open.mockResolvedValue(null)
    await useSessionStore.getState().openDialog()
    expect(open).toHaveBeenCalledWith(
      expect.objectContaining({
        filters: [
          {
            name: 'Fonts and font sources',
            extensions: expect.arrayContaining([
              'otf',
              'glyphs',
              'sfd',
              'pfa',
              'pfb',
            ]),
          },
        ],
      }),
    )
  })
})
