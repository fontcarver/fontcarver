import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  setTransport,
  type EditApplyParams,
  type KernPair,
  type KerningResolution,
} from '../../api'
import { createMockTransport, noChange } from '../../test/mockTransport'
import { useUiStore } from '../../stores/uiStore'
import { useTextKerning, type UseTextKerningOptions } from './useTextKerning'

const groups = { left: 'public.kern1.A', right: 'public.kern2.V' }
const comma = {
  key: ',',
  code: 'Comma',
  ctrlKey: false,
  altKey: false,
  metaKey: false,
  shiftKey: false,
}
function setup(
  leftGroup: string | null = groups.left,
  rightGroup: string | null = groups.right,
  gates: { query?: Promise<void> } = {},
) {
  const records: KernPair[] = []
  let token = 0
  const transport = createMockTransport({
    'kerning.resolve': async ({
      master,
      left,
      right,
    }): Promise<KerningResolution> => {
      await gates.query
      const matching = records.filter(
        (record) =>
          ('glyph' in record.left
            ? record.left.glyph === left
            : record.left.group === leftGroup) &&
          ('glyph' in record.right
            ? record.right.glyph === right
            : record.right.group === rightGroup),
      )
      const rule =
        matching.sort(
          (a, b) =>
            Number('group' in a.left) +
            Number('group' in a.right) -
            Number('group' in b.left) -
            Number('group' in b.right),
        )[0] ?? null
      return {
        status: { generation: 1, editable: true, reason: null },
        master,
        left,
        right,
        left_group: leftGroup,
        right_group: rightGroup,
        value: rule?.value ?? 0,
        rule,
        records: matching,
      }
    },
    'kerning.members': () => ({
      status: { generation: 1, editable: true, reason: null },
      total: 2,
      items: ['A', 'V'],
    }),
    'doc.edit_begin': () => ++token,
    'doc.edit_apply': ({ command }) => {
      if (!('SetHorizontalKerning' in command))
        throw new Error('unexpected command')
      const { left, right, value } = command.SetHorizontalKerning
      const index = records.findIndex(
        (record) =>
          JSON.stringify(record.left) === JSON.stringify(left) &&
          JSON.stringify(record.right) === JSON.stringify(right),
      )
      if (index >= 0) records.splice(index, 1)
      if (value !== null) records.push({ left, right, value })
      return { ...noChange(), layout: true }
    },
    'doc.edit_end': () => null,
  })
  setTransport(transport)
  const options: UseTextKerningOptions = {
    doc: 41,
    master: 'm1',
    generation: 1,
    selectedPair: { left: 'A', right: 'V' },
    punctuationShortcuts: true,
  }
  const hook = renderHook(
    (props: UseTextKerningOptions) => useTextKerning(props),
    { initialProps: options },
  )
  return { ...hook, options, records, transport }
}

afterEach(() => {
  setTransport(undefined)
  useUiStore.setState({ modalCount: 0 })
})

describe('kerning class defaults and exceptions', () => {
  it.each([
    [groups.left, groups.right, 'group_group'],
    [groups.left, null, 'group_glyph'],
    [null, groups.right, 'glyph_group'],
    [null, null, 'glyph_glyph'],
  ])('defaults %s / %s to %s', async (left, right, scope) => {
    const hook = setup(left, right)
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    expect(hook.result.current.panel.scope).toBe(scope)
    hook.unmount()
  })

  it('preserves the explicit scope through reload and reselection, then defaults a new pair', async () => {
    const hook = setup()
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    await act(async () => {
      await hook.result.current.panel.setScope('glyph_group')
    })
    hook.rerender({ ...hook.options, generation: 2 })
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    await act(async () => {
      await hook.result.current.panel.setPair({ left: 'A', right: 'V' })
    })
    expect(hook.result.current.panel.scope).toBe('glyph_group')
    await act(async () => {
      await hook.result.current.panel.setPair({ left: 'T', right: 'o' })
    })
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    expect(hook.result.current.panel.scope).toBe('group_group')
    hook.unmount()
  })

  it('captures each held exception separately and restores the class target when modifiers lift', async () => {
    const hook = setup()
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    for (const modifiers of [
      {},
      { ctrlKey: true },
      { altKey: true },
      { ctrlKey: true, altKey: true },
      {},
    ]) {
      await act(async () => {
        expect(
          hook.result.current.onKerningKey({ ...comma, ...modifiers }),
        ).toBe(true)
      })
    }
    await act(async () => {
      await hook.result.current.finish()
    })
    const applied = hook.transport.calls
      .filter((call) => call.method === 'doc.edit_apply')
      .map((call) => (call.params as EditApplyParams).command)
    expect(applied).toEqual([
      {
        SetHorizontalKerning: {
          master: 'm1',
          left: { group: groups.left },
          right: { group: groups.right },
          value: -1,
        },
      },
      {
        SetHorizontalKerning: {
          master: 'm1',
          left: { glyph: 'A' },
          right: { group: groups.right },
          value: -2,
        },
      },
      {
        SetHorizontalKerning: {
          master: 'm1',
          left: { group: groups.left },
          right: { glyph: 'V' },
          value: -3,
        },
      },
      {
        SetHorizontalKerning: {
          master: 'm1',
          left: { glyph: 'A' },
          right: { glyph: 'V' },
          value: -3,
        },
      },
      {
        SetHorizontalKerning: {
          master: 'm1',
          left: { group: groups.left },
          right: { group: groups.right },
          value: -2,
        },
      },
    ])
    expect(hook.result.current.panel.scope).toBe('group_group')
    hook.unmount()
  })

  it('keeps explicit zero and removal on the chosen class rule', async () => {
    const hook = setup()
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    act(() => hook.result.current.panel.setValue('0'))
    await act(async () => {
      await hook.result.current.panel.commit()
    })
    expect(hook.records).toEqual([
      {
        left: { group: groups.left },
        right: { group: groups.right },
        value: 0,
      },
    ])
    await act(async () => {
      await hook.result.current.panel.remove()
    })
    expect(hook.records).toEqual([])
    hook.unmount()
  })

  it('does not reserve punctuation outside the enabled kerning context', async () => {
    const hook = setup()
    hook.rerender({ ...hook.options, punctuationShortcuts: false })
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    expect(hook.result.current.onKerningKey(comma)).toBe(false)
    expect(
      hook.transport.calls.some((call) => call.method === 'doc.edit_begin'),
    ).toBe(false)
    hook.unmount()
  })

  it('retains class repeats through a generation reload without creating a glyph exception', async () => {
    const gates: { query?: Promise<void> } = {}
    const hook = setup(groups.left, groups.right, gates)
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    act(() => {
      hook.result.current.onKerningKey(comma)
    })
    await vi.waitFor(() => expect(hook.records[0]?.value).toBe(-1))
    let release!: () => void
    gates.query = new Promise<void>((resolve) => {
      release = resolve
    })
    hook.rerender({ ...hook.options, generation: 2 })
    expect(hook.result.current.panel.editable).toBe(false)
    expect(hook.result.current.panel.scope).toBe('group_group')
    act(() => {
      expect(
        hook.result.current.onKerningKey({ ...comma, shiftKey: true }),
      ).toBe(true)
    })
    await act(async () => {
      await hook.result.current.finish()
      release()
    })
    expect(hook.records).toEqual([
      {
        left: { group: groups.left },
        right: { group: groups.right },
        value: -11,
      },
    ])
    hook.unmount()
  })

  it('keeps punctuation input and modal ownership intact', async () => {
    const hook = setup()
    await vi.waitFor(() =>
      expect(hook.result.current.panel.editable).toBe(true),
    )
    expect(
      hook.result.current.onKerningKey({
        ...comma,
        target: document.createElement('input'),
      }),
    ).toBe(false)
    useUiStore.setState({ modalCount: 1 })
    expect(hook.result.current.onKerningKey(comma)).toBe(false)
    expect(
      hook.transport.calls.some((call) => call.method === 'doc.edit_begin'),
    ).toBe(false)
    hook.unmount()
  })
})
