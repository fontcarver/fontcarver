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

  it('opens project folders with the native directory picker', async () => {
    setTransport(transportOfKind('tauri'))
    open.mockResolvedValue('/fonts/Family.fcfp')

    expect(await choosePath({ kind: 'open', directory: true })).toEqual({
      kind: 'chosen',
      path: '/fonts/Family.fcfp',
    })
    expect(open).toHaveBeenCalledWith({
      multiple: false,
      directory: true,
      filters: undefined,
    })
  })

  it('does not open a document after cancelling the project folder picker', async () => {
    const transport = transportOfKind('tauri')
    const call = vi.spyOn(transport, 'call')
    setTransport(transport)
    open.mockResolvedValue(null)

    await useSessionStore.getState().openProjectDialog()

    expect(open).toHaveBeenCalledWith(
      expect.objectContaining({ directory: true }),
    )
    expect(call).not.toHaveBeenCalled()
  })

  it('opens the project directory selected in the native picker', async () => {
    setTransport(transportOfKind('tauri'))
    open.mockResolvedValue('/fonts/Family.fcfp')
    const openDocument = vi
      .spyOn(useSessionStore.getState(), 'open')
      .mockResolvedValue(null)

    try {
      await useSessionStore.getState().openProjectDialog()

      expect(openDocument).toHaveBeenCalledWith('/fonts/Family.fcfp')
    } finally {
      openDocument.mockRestore()
    }
  })

  it('offers directory projects after the default native save formats', async () => {
    setTransport(transportOfKind('tauri'))
    save.mockResolvedValue(null)

    await useSessionStore.getState().requestSaveAs(1)

    expect(save).toHaveBeenCalledWith({
      defaultPath: undefined,
      filters: [
        { name: 'FontCarver document', extensions: ['fcf', 'fcf.gz', 'fcfp'] },
      ],
    })
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
              'fcfp',
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
