import { afterEach, describe, expect, it, vi } from 'vitest'
import { setTransport, type EditApplyParams } from '../../api'
import { afterDocumentEdits } from '../../editor/editSession'
import { createEditingTransport } from '../../test/mockTransport'
import { KerningEditRunner, kerningShortcut } from './kerningEdits'

const target = {
  doc: 1,
  master: 'm1',
  left: { glyph: 'A' },
  right: { glyph: 'V' },
}
const key = {
  key: 'ArrowLeft',
  ctrlKey: true,
  metaKey: false,
  altKey: true,
  shiftKey: false,
}
function deferred() {
  let resolve!: () => void
  const promise = new Promise<void>((done) => {
    resolve = done
  })
  return { promise, resolve }
}

afterEach(() => {
  setTransport(undefined)
  vi.useRealTimers()
})

describe('kerningShortcut', () => {
  it('uses platform Mod+Alt and the documented 1/10/100-unit steps', () => {
    expect(kerningShortcut(key, false)).toBe(-1)
    expect(
      kerningShortcut({ ...key, key: 'ArrowRight', shiftKey: true }, false),
    ).toBe(10)
    expect(kerningShortcut({ ...key, metaKey: true }, true)).toBe(-100)
    expect(
      kerningShortcut({ ...key, metaKey: true, ctrlKey: false }, true),
    ).toBe(-1)
    expect(kerningShortcut({ ...key, altKey: false }, false)).toBeNull()
    expect(kerningShortcut(key, true)).toBeNull()
    expect(kerningShortcut({ ...key, key: 'ArrowUp' }, false)).toBeNull()
  })
})

describe('KerningEditRunner', () => {
  it('refreshes the baseline after earlier work and captures queued repeats before closing', async () => {
    const gate = deferred()
    const mock = createEditingTransport()
    setTransport(mock)
    const refresh = vi.fn(async () => -80)
    const run = new KerningEditRunner({
      ...target,
      label: 'Kerning',
      after: gate.promise,
      refreshStart: refresh,
      onEnd: vi.fn(),
      onError: vi.fn(),
    })
    run.press(-1)
    run.press(10)
    const closing = run.end()
    expect(mock.calls).toHaveLength(0)
    expect(refresh).not.toHaveBeenCalled()
    gate.resolve()
    await closing
    expect(mock.calls.map((item) => item.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_apply',
      'doc.edit_end',
    ])
    expect((mock.calls[1].params as EditApplyParams).command).toEqual({
      SetHorizontalKerning: {
        master: 'm1',
        left: { glyph: 'A' },
        right: { glyph: 'V' },
        value: -71,
      },
    })
  })

  it('ends after 500 ms and registers its closure before later document commands', async () => {
    vi.useFakeTimers()
    const gate = deferred()
    const mock = createEditingTransport({
      hold: { on: 'doc.edit_end', until: gate.promise },
    })
    setTransport(mock)
    const ended = vi.fn()
    const run = new KerningEditRunner({
      ...target,
      label: 'Kerning',
      refreshStart: async () => 0,
      onEnd: ended,
      onError: vi.fn(),
    })
    run.press(-1)
    await vi.advanceTimersByTimeAsync(499)
    expect(run.active).toBe(true)
    await vi.advanceTimersByTimeAsync(1)
    expect(run.active).toBe(false)
    const command = vi.fn(async () => undefined)
    const later = afterDocumentEdits(1, command)
    expect(command).not.toHaveBeenCalled()
    gate.resolve()
    await later
    expect(ended).toHaveBeenCalledOnce()
    expect(command).toHaveBeenCalledOnce()
  })

  it('distinguishes explicit zero from removing the stored record', async () => {
    const mock = createEditingTransport()
    setTransport(mock)
    for (const value of [0, null]) {
      const run = new KerningEditRunner({
        ...target,
        label: 'Kerning',
        refreshStart: async () => -80,
        onEnd: vi.fn(),
        onError: vi.fn(),
      })
      run.set(value)
      await run.end()
    }
    const commands = mock.calls
      .filter((item) => item.method === 'doc.edit_apply')
      .map((item) => (item.params as EditApplyParams).command)
    expect(commands).toMatchObject([
      { SetHorizontalKerning: { value: 0 } },
      { SetHorizontalKerning: { value: null } },
    ])
  })

  it('reports one baseline failure and closes without applying guessed values', async () => {
    const mock = createEditingTransport()
    setTransport(mock)
    const onError = vi.fn()
    const run = new KerningEditRunner({
      ...target,
      label: 'Kerning',
      refreshStart: async () => {
        throw new Error('read failed')
      },
      onEnd: vi.fn(),
      onError,
    })
    run.press(-1)
    await run.end()
    expect(mock.calls.map((item) => item.method)).toEqual([
      'doc.edit_begin',
      'doc.edit_end',
    ])
    expect(onError).toHaveBeenCalledOnce()
  })
})
