import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  call,
  setTransport,
  type EditApplyParams,
  type KerningResolution,
} from '../../api'
import { afterDocumentEdits } from '../../editor/editSession'
import { suspendDocumentEditParticipants } from '../../editor/documentEditParticipants'
import * as commands from '../../shell/commands'
import { useUiStore } from '../../stores/uiStore'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { useTextKerning, type UseTextKerningOptions } from './useTextKerning'

const key = {
  key: 'ArrowLeft',
  ctrlKey: true,
  metaKey: true,
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
function setup(
  gates: {
    prepare?: Promise<void>
    end?: Promise<void>
    oldQuery?: Promise<void>
    leftGroup?: string
  } = {},
) {
  let finish = () => Promise.resolve()
  let active: number | null = null
  let token = 0
  const values = new Map<string, number>()
  const options: UseTextKerningOptions = {
    doc: 1,
    master: 'm1',
    generation: 1,
    selectedPair: { left: 'A', right: 'V' },
    prepare: () => Promise.all([gates.prepare, finish()]).then(() => undefined),
  }
  const transport = createMockTransport({
    'kerning.status': () => ({ generation: 1, editable: true, reason: null }),
    'kerning.resolve': async ({
      doc,
      master,
      left,
      right,
    }): Promise<KerningResolution> => {
      if (left === 'A') await gates.oldQuery
      const value = values.get(JSON.stringify([doc, master, left, right]))
      const rule =
        value === undefined
          ? null
          : { left: { glyph: left }, right: { glyph: right }, value }
      return {
        status: { generation: 1, editable: true, reason: null },
        master,
        left,
        right,
        left_group: gates.leftGroup ?? null,
        right_group: null,
        value: value ?? 0,
        rule,
        records: rule ? [rule] : [],
      }
    },
    'kerning.members': () => ({
      status: { generation: 1, editable: true, reason: null },
      total: 2,
      items: ['A', 'a'],
    }),
    'doc.edit_begin': () => {
      if (active !== null) throw new Error('begin overtook old run')
      active = ++token
      return token
    },
    'doc.edit_apply': ({ doc, token: applying, command }) => {
      if (active !== applying) throw new Error('apply overtaken')
      if (!('SetHorizontalKerning' in command))
        throw new Error('unexpected command')
      const { master, left, right, value } = command.SetHorizontalKerning
      if (!('glyph' in left) || !('glyph' in right))
        throw new Error('unexpected scope')
      const identity = JSON.stringify([doc, master, left.glyph, right.glyph])
      if (value === null) values.delete(identity)
      else values.set(identity, value)
      return { ...noChange(), layout: true }
    },
    'doc.edit_end': async ({ token: ending }) => {
      await gates.end
      if (active !== ending) throw new Error('end overtaken')
      active = null
      return null
    },
    'doc.apply': () => {
      if (active !== null) throw new Error('command overtook close')
      return { ...noChange(), layout: false }
    },
  })
  setTransport(transport)
  const hook = renderHook(
    (props: UseTextKerningOptions) => useTextKerning(props),
    { initialProps: options },
  )
  finish = hook.result.current.finish
  return { ...hook, transport, values, options }
}

afterEach(() => {
  setTransport(undefined)
  useUiStore.setState({ modalCount: 0 })
  vi.restoreAllMocks()
})

it('does not accept numeric or shortcut drafts while the document is closing', async () => {
  const hook = setup()
  await vi.waitFor(() => expect(hook.result.current.panel.editable).toBe(true))
  const resume = suspendDocumentEditParticipants(1)
  try {
    act(() => {
      hook.result.current.panel.setValue('-80')
      hook.result.current.onKerningKey(key)
    })
    await hook.result.current.finish()
    expect(
      hook.transport.calls.some((call) => call.method === 'doc.edit_begin'),
    ).toBe(false)
    expect(hook.result.current.panel.value).toBe('0')
  } finally {
    resume()
  }
})

describe('useTextKerning', () => {
  it('drains a numeric draft through reentrant preparation before save', async () => {
    const gate = deferred()
    const hook = setup({ prepare: gate.promise })
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    act(() => hook.result.current.panel.setValue('-80'))
    const closing = hook.result.current.finish()
    const saving = afterDocumentEdits(1, () =>
      call('doc.apply', {
        doc: 1,
        command: { Batch: [] },
        label: 'Save barrier',
      }),
    )
    expect(
      hook.transport.calls.filter((item) => item.method === 'doc.edit_begin'),
    ).toHaveLength(0)
    gate.resolve()
    await act(async () => {
      await closing
      await saving
    })
    expect(hook.values.get('[1,"m1","A","V"]')).toBe(-80)
    expect(
      hook.transport.calls
        .filter((item) => item.method.startsWith('doc.'))
        .map((item) => item.method),
    ).toEqual(['doc.edit_begin', 'doc.edit_apply', 'doc.edit_end', 'doc.apply'])
    hook.unmount()
  })

  it('keeps numeric drafts on the captured master when a new master arrives', async () => {
    const hook = setup()
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    act(() => hook.result.current.panel.setValue('0'))
    hook.rerender({ ...hook.options, master: 'm2' })
    await act(async () => {
      await hook.result.current.finish()
    })
    expect(hook.values.get('[1,"m1","A","V"]')).toBe(0)
    expect(hook.values.has('[1,"m2","A","V"]')).toBe(false)
    hook.unmount()
  })

  it('drains a numeric draft staged while a previous numeric edit is closing', async () => {
    const end = deferred()
    const hook = setup({ end: end.promise })
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    act(() => {
      hook.result.current.panel.setValue('-80')
    })
    const first = hook.result.current.finish()
    await vi.waitFor(() =>
      expect(
        hook.transport.calls.some((item) => item.method === 'doc.edit_end'),
      ).toBe(true),
    )
    act(() => {
      hook.result.current.panel.setValue('-90')
    })
    const second = hook.result.current.finish()
    expect(
      hook.transport.calls.filter((item) => item.method === 'doc.edit_begin'),
    ).toHaveLength(1)
    end.resolve()
    await act(async () => {
      await first
      await second
    })
    expect(hook.values.get('[1,"m1","A","V"]')).toBe(-90)
    expect(
      hook.transport.calls.filter((item) => item.method === 'doc.edit_end'),
    ).toHaveLength(2)
    hook.unmount()
  })

  it('ignores an old pair query after the selected pair changes', async () => {
    const old = deferred()
    const hook = setup({ oldQuery: old.promise })
    hook.rerender({ ...hook.options, selectedPair: { left: 'T', right: 'o' } })
    await vi.waitFor(() =>
      expect(hook.result.current.panel.resolution?.left).toBe('T'),
    )
    await act(async () => {
      old.resolve()
      await old.promise
    })
    expect(hook.result.current.panel.resolution?.left).toBe('T')
    hook.unmount()
  })

  it('retains a keyboard repeat while the previous run closes and generation reload is pending', async () => {
    const end = deferred()
    const reload = deferred()
    const gates = {
      end: end.promise,
      oldQuery: undefined as Promise<void> | undefined,
    }
    const hook = setup(gates)
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    act(() => {
      expect(hook.result.current.onKerningKey(key)).toBe(true)
    })
    const first = hook.result.current.finish()
    await vi.waitFor(() =>
      expect(
        hook.transport.calls.some((item) => item.method === 'doc.edit_end'),
      ).toBe(true),
    )
    gates.oldQuery = reload.promise
    hook.rerender({ ...hook.options, generation: 2 })
    expect(hook.result.current.panel.editable).toBe(false)
    act(() => {
      hook.result.current.onKerningKey({ ...key, shiftKey: true })
    })
    const second = hook.result.current.finish()
    end.resolve()
    reload.resolve()
    await act(async () => {
      await first
      await second
    })
    expect(hook.values.get('[1,"m1","A","V"]')).toBe(-11)
    const applied = hook.transport.calls
      .filter((item) => item.method === 'doc.edit_apply')
      .map((item) => (item.params as EditApplyParams).command)
    expect(applied).toMatchObject([
      { SetHorizontalKerning: { value: -1 } },
      { SetHorizontalKerning: { value: -11 } },
    ])
    hook.unmount()
  })

  it('retains active-run repeats while its own apply causes a generation reload', async () => {
    const reload = deferred()
    const gates = { oldQuery: undefined as Promise<void> | undefined }
    const hook = setup(gates)
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    act(() => {
      hook.result.current.onKerningKey(key)
    })
    await vi.waitFor(() => expect(hook.values.get('[1,"m1","A","V"]')).toBe(-1))
    gates.oldQuery = reload.promise
    hook.rerender({ ...hook.options, generation: 2 })
    expect(hook.result.current.panel.editable).toBe(false)
    act(() => {
      expect(hook.result.current.onKerningKey({ ...key, shiftKey: true })).toBe(
        true,
      )
    })
    await act(async () => {
      await hook.result.current.finish()
      reload.resolve()
    })
    expect(hook.values.get('[1,"m1","A","V"]')).toBe(-11)
    hook.unmount()
  })

  it('rejects a numeric draft if the captured group membership changed before apply', async () => {
    const report = vi
      .spyOn(commands, 'reportError')
      .mockImplementation(() => undefined)
    const gates = { leftGroup: 'public.kern1.A' }
    const hook = setup(gates)
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    await act(async () => {
      await hook.result.current.panel.setScope('group_glyph')
    })
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    act(() => {
      hook.result.current.panel.setValue('-80')
    })
    gates.leftGroup = 'public.kern1.New'
    await act(async () => {
      await hook.result.current.finish()
    })
    expect(
      hook.transport.calls.filter((item) => item.method === 'doc.edit_apply'),
    ).toHaveLength(0)
    expect(report).toHaveBeenCalledOnce()
    expect(report.mock.calls[0][0]).toMatchObject({ code: 'stale_generation' })
    hook.unmount()
  })

  it('leaves text inputs and modals untouched and cancels invalid or escaped drafts', async () => {
    const hook = setup()
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    expect(
      hook.result.current.onKerningKey({
        ...key,
        target: document.createElement('input'),
      }),
    ).toBe(false)
    useUiStore.setState({ modalCount: 1 })
    expect(hook.result.current.onKerningKey(key)).toBe(false)
    act(() => {
      hook.result.current.panel.setValue('NaN')
    })
    await act(async () => {
      await hook.result.current.panel.commit()
    })
    expect(hook.result.current.panel.invalid).toBe(true)
    act(() => {
      hook.result.current.panel.setValue('-80')
      hook.result.current.panel.cancel()
    })
    await act(async () => {
      await hook.result.current.finish()
    })
    expect(
      hook.transport.calls.filter((item) => item.method.startsWith('doc.')),
    ).toHaveLength(0)
    hook.unmount()
  })
})
