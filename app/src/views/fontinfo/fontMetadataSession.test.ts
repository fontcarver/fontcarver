import { afterEach, describe, expect, it } from 'vitest'
import { setTransport, type FontInfo } from '../../api'
import {
  flushDocumentEditParticipants,
  hasPendingDocumentDraft,
  suspendDocumentEditParticipants,
} from '../../editor/documentEditParticipants'
import { makeFontInfo, makeFontInfoView } from '../../test/fontFixtures'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { FontMetadataSession } from './fontMetadataSession'

afterEach(() => setTransport(undefined))

function setup(initial = makeFontInfo()) {
  let info = structuredClone(initial)
  let failApply = false
  let failEnd = false
  const transport = createMockTransport({
    'doc.font_info': () => makeFontInfoView({ info: structuredClone(info) }),
    'doc.edit_begin': () => 71,
    'doc.edit_apply': ({ token, command }) => {
      expect(token).toBe(71)
      if (failApply) throw new Error('write failed')
      if ('SetFontInfo' in command) info = command.SetFontInfo.info
      else throw new Error('unexpected command')
      return noChange()
    },
    'doc.edit_end': () => {
      if (failEnd) throw new Error('end failed')
      return null
    },
  })
  setTransport(transport)
  return {
    session: new FontMetadataSession(981, 'Edit metadata'),
    transport,
    info: () => info,
    externalEdit: (patch: Partial<FontInfo>) => {
      info = { ...info, ...patch }
    },
    failApply: (value: boolean) => {
      failApply = value
    },
    failEnd: (value: boolean) => {
      failEnd = value
    },
  }
}

describe('FontMetadataSession', () => {
  it('merges only changed fields inside one transaction, preserving fresh imported metadata', async () => {
    const test = setup()
    await test.session.load()
    test.session.edit({ family_name: 'New Family', units_per_em: '1000' })
    const names: FontInfo['names'] = [
      { name_id: 1, language: 'de', text: 'Familie', platform: 'Windows' },
    ]
    test.externalEdit({
      note: 'Concurrent note',
      names,
      user_data: { opaque: 42 },
    })
    await test.session.flush()
    expect(test.info()).toMatchObject({
      family_name: 'New Family',
      units_per_em: 1000,
      note: 'Concurrent note',
      names,
      user_data: { opaque: 42 },
    })
    expect(test.transport.calls.map((call) => call.method)).toEqual([
      'doc.font_info',
      'doc.edit_begin',
      'doc.font_info',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    expect(test.session.snapshot().dirty).toBe(false)
  })

  it('rejects a conflicting field and retains the draft until explicit discard', async () => {
    const test = setup()
    await test.session.load()
    test.session.edit({ family_name: 'My family' })
    test.externalEdit({ family_name: 'Their family' })
    await expect(test.session.flush()).rejects.toThrow(/changed elsewhere/)
    expect(test.session.snapshot().draft?.family_name).toBe('My family')
    expect(test.session.snapshot().dirty).toBe(true)
    expect(test.info().family_name).toBe('Their family')
    expect(test.transport.calls.at(-1)?.method).toBe('doc.edit_end')
    await test.session.load()
    expect(test.session.snapshot().draft?.family_name).toBe('My family')
    await test.session.discard()
    expect(test.session.snapshot().draft?.family_name).toBe('Their family')
    expect(test.session.snapshot().dirty).toBe(false)
  })

  it('joins save/history handover, preserves apply failures and rejects input while suspended', async () => {
    const test = setup()
    const dispose = test.session.connect()
    try {
      await test.session.load()
      test.session.edit({ copyright: 'New copyright' })
      expect(hasPendingDocumentDraft(981)).toBe(true)
      const release = suspendDocumentEditParticipants(981)
      test.session.edit({ copyright: 'Ignored' })
      expect(test.session.snapshot().draft?.copyright).toBe('New copyright')
      release()
      test.failApply(true)
      await expect(flushDocumentEditParticipants(981)).rejects.toThrow(
        'write failed',
      )
      expect(test.session.snapshot().dirty).toBe(true)
      expect(test.transport.calls.at(-1)?.method).toBe('doc.edit_end')
      test.failApply(false)
      await flushDocumentEditParticipants(981)
      expect(test.info().copyright).toBe('New copyright')
      expect(hasPendingDocumentDraft(981)).toBe(false)
    } finally {
      dispose()
    }
  })

  it('retains a failed end draft and retries without a duplicate apply', async () => {
    const test = setup()
    await test.session.load()
    test.session.edit({ style_name: 'Bold' })
    test.failEnd(true)
    await expect(test.session.flush()).rejects.toThrow('end failed')
    expect(test.session.snapshot().dirty).toBe(true)
    test.failEnd(false)
    await test.session.flush()
    expect(test.session.snapshot().dirty).toBe(false)
    expect(
      test.transport.calls.filter((call) => call.method === 'doc.edit_apply'),
    ).toHaveLength(1)
  })

  it.each([
    { units_per_em: '' },
    { units_per_em: '0' },
    { units_per_em: '65536' },
    { units_per_em: '1.5' },
    { units_per_em: 'NaN' },
    { version_major: '-1' },
    { version_minor: '4294967296' },
    { version_minor: '3.1' },
    { family_name: ' ' },
    { style_name: '' },
  ])(
    'validates changed values before opening a transaction: %j',
    async (patch) => {
      const test = setup()
      await test.session.load()
      test.session.edit(patch)
      await expect(test.session.flush()).rejects.toThrow()
      expect(test.session.snapshot().dirty).toBe(true)
      expect(test.transport.calls).toHaveLength(1)
    },
  )

  it('preserves unchanged empty optional strings and clears changed optional fields to null', async () => {
    const test = setup(makeFontInfo({ note: '', trademark: 'Trademark' }))
    await test.session.load()
    test.session.edit({
      trademark: '',
      version_major: '4',
      version_minor: '12',
    })
    await test.session.flush()
    expect(test.info()).toMatchObject({
      note: '',
      trademark: null,
      version_major: 4,
      version_minor: 12,
    })
  })

  it('does not apply an unchanged or reverted draft', async () => {
    const test = setup()
    await test.session.load()
    test.session.edit({ family_name: 'Changed' })
    test.session.edit({ family_name: 'DejaVu Sans' })
    expect(test.session.snapshot().dirty).toBe(false)
    await test.session.flush()
    expect(test.transport.calls).toHaveLength(1)
  })

  it('coalesces concurrent flushes and blocks changes until the write settles', async () => {
    const test = setup()
    await test.session.load()
    test.session.edit({ style_name: 'Bold' })
    const first = test.session.flush()
    const second = test.session.flush()
    test.session.edit({ style_name: 'Italic' })
    await Promise.all([first, second])
    expect(test.info().style_name).toBe('Bold')
    expect(
      test.transport.calls.filter((call) => call.method === 'doc.edit_begin'),
    ).toHaveLength(1)
  })

  it('keeps a draft when edit_begin fails without trying to close an unknown token', async () => {
    const session = new FontMetadataSession(983, 'Metadata')
    const transport = createMockTransport({
      'doc.font_info': () => makeFontInfoView(),
      'doc.edit_begin': () => {
        throw new Error('begin failed')
      },
    })
    setTransport(transport)
    await session.load()
    session.edit({ style_name: 'Bold' })
    await expect(session.flush()).rejects.toThrow('begin failed')
    expect(session.snapshot().draft?.style_name).toBe('Bold')
    expect(session.snapshot().dirty).toBe(true)
    expect(transport.calls.at(-1)?.method).toBe('doc.edit_begin')
  })

  it('closes a token after a fresh-read failure and keeps the first failure', async () => {
    let reads = 0
    const session = new FontMetadataSession(984, 'Metadata')
    const transport = createMockTransport({
      'doc.font_info': () => {
        if (++reads > 1) throw new Error('read failed')
        return makeFontInfoView()
      },
      'doc.edit_begin': () => 73,
      'doc.edit_end': () => {
        throw new Error('secondary failure')
      },
    })
    setTransport(transport)
    await session.load()
    session.edit({ style_name: 'Bold' })
    await expect(session.flush()).rejects.toThrow('read failed')
    expect(session.snapshot().dirty).toBe(true)
    expect(transport.calls.at(-1)?.method).toBe('doc.edit_end')
  })

  it('retains the draft if another command invalidates the token after the fresh read', async () => {
    let info = makeFontInfo()
    let active: number | null = null
    let reads = 0
    const session = new FontMetadataSession(985, 'Metadata')
    const transport = createMockTransport({
      'doc.edit_begin': () => {
        active = 74
        return active
      },
      'doc.font_info': () => {
        const view = makeFontInfoView({ info: structuredClone(info) })
        if (++reads === 2) {
          info = { ...info, note: 'Other command' }
          active = null
        }
        return view
      },
      'doc.edit_apply': ({ token, command }) => {
        if (token !== active) throw new Error('InvalidEditToken')
        if ('SetFontInfo' in command) info = command.SetFontInfo.info
        return noChange()
      },
      'doc.edit_end': () => null,
    })
    setTransport(transport)
    await session.load()
    session.edit({ style_name: 'Bold' })
    await expect(session.flush()).rejects.toThrow('InvalidEditToken')
    expect(info.note).toBe('Other command')
    expect(info.style_name).toBe('Book')
    expect(session.snapshot().draft?.style_name).toBe('Bold')
    await session.flush()
    expect(info).toMatchObject({ note: 'Other command', style_name: 'Bold' })
  })

  it('ignores out-of-order hydration responses', async () => {
    let first:
      ((value: ReturnType<typeof makeFontInfoView>) => void) | undefined
    let reads = 0
    const session = new FontMetadataSession(986, 'Metadata')
    setTransport(
      createMockTransport({
        'doc.font_info': () => {
          if (++reads === 1)
            return new Promise((resolve) => {
              first = resolve
            })
          return makeFontInfoView({
            info: makeFontInfo({ style_name: 'Latest' }),
          })
        },
      }),
    )
    const older = session.load()
    await session.load()
    first?.(makeFontInfoView())
    await older
    expect(session.snapshot().draft?.style_name).toBe('Latest')
    expect(session.snapshot().busy).toBe(false)
  })
})
