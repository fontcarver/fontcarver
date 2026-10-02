import { act, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type ApplyParams, type Layer } from '../../../api'
import { setCommandErrorReporter } from '../../../shell/commands'
import {
  makeGlyph,
  makeFontInfoView,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
  makeMasterMetrics,
} from '../../../test/fontFixtures'
import {
  createMockTransport,
  noChange,
  type MockHandlers,
  type MockTransport,
} from '../../../test/mockTransport'
import { buildScene } from '../glyphScene'

import { VerticalMetricsPanel } from './VerticalMetricsPanel'

function show(
  options: {
    layer?: Partial<Layer>
    bbox?: [number, number, number, number] | null
    reject?: boolean
    apply?: MockHandlers['doc.apply']
    readOutline?: MockHandlers['glyphs.outline']
  } = {},
) {
  const layer = makeLayer(options.layer)
  const glyph = makeGlyph({ layers: [layer] })
  const stored = new Map([[1, glyph]])
  const master = makeMaster({
    metrics: makeMasterMetrics({
      default_vertical_advance: 1100,
      default_vertical_origin: 880,
    }),
  })
  const scene = buildScene({
    detail: makeGlyphDetail(glyph),
    cell: null,
    master,
    masterId: 'm1',
    unitsPerEm: 1000,
  })
  const reload = vi.fn()
  const errors = vi.fn()
  const mock = createMockTransport({
    'glyphs.detail': ({ doc }) => makeGlyphDetail(stored.get(doc) ?? glyph),
    'glyphs.outline': options.readOutline ?? (() => outline),
    'doc.font_info': () => makeFontInfoView({ masters: [master] }),
    'doc.apply': async (params) => {
      if (options.reject) throw new Error('Vertical advance is out of range')
      const change = await (options.apply?.(params) ?? noChange([42]))
      if ('SetVerticalMetrics' in params.command) {
        const { advance, origin } = params.command.SetVerticalMetrics
        stored.set(params.doc, {
          ...glyph,
          layers: [
            { ...layer, vertical_width: advance, vertical_origin: origin },
          ],
        })
      }
      return change
    },
  })
  setTransport(mock)
  setCommandErrorReporter(errors)
  const outline = makeLayerOutline(layer, {
    bbox: options.bbox === undefined ? [40, -100, 560, 700] : options.bbox,
  })
  const view = render(
    <VerticalMetricsPanel
      gid={42}
      target={{ doc: 1, glyph: glyph.name, layer: layer.id }}
      glyph={glyph}
      scene={scene}
      outline={outline}
      reload={reload}
    />,
  )
  return { mock, reload, errors, layer, glyph, scene, outline, view, stored }
}

function edits(mock: MockTransport): ApplyParams[] {
  return mock.calls
    .filter((call) => call.method === 'doc.apply')
    .map((call) => call.params as ApplyParams)
}

async function enter(label: string, value: string) {
  const user = userEvent.setup()
  const input = screen.getByRole('textbox', { name: label })
  await user.clear(input)
  await user.type(input, `${value}{Enter}`)
}

afterEach(() => {
  setTransport(undefined)
  setCommandErrorReporter(() => undefined)
})

describe('vertical metrics inspector', () => {
  it('derives both sidebearings from effective master metrics and resolved ink bounds', () => {
    show()
    expect(screen.getByRole('textbox', { name: 'TSB' })).toHaveValue('180')
    expect(screen.getByRole('textbox', { name: 'BSB' })).toHaveValue('120')
    expect(screen.getByRole('textbox', { name: 'vert. origin' })).toHaveValue(
      '880',
    )
    expect(screen.getByRole('textbox', { name: 'vert. advance' })).toHaveValue(
      '1100',
    )
    expect(
      screen.getByRole('button', { name: 'Use master vertical metrics' }),
    ).toBeDisabled()
  })

  it('changes origin for TSB while retaining inherited advance', async () => {
    const { mock, glyph, layer } = show()
    await enter('TSB', '210')
    await waitFor(() => expect(edits(mock)).toHaveLength(1))
    expect(edits(mock)[0]?.command).toEqual({
      SetVerticalMetrics: {
        glyph: glyph.name,
        layer: layer.id,
        advance: null,
        origin: 910,
      },
    })
  })

  it('changes advance for BSB while retaining explicit zero origin', async () => {
    const { mock, glyph, layer } = show({ layer: { vertical_origin: 0 } })
    await enter('BSB', '250')
    await waitFor(() => expect(edits(mock)).toHaveLength(1))
    expect(edits(mock)[0]?.command).toEqual({
      SetVerticalMetrics: {
        glyph: glyph.name,
        layer: layer.id,
        advance: 350,
        origin: 0,
      },
    })
  })

  it('carries the other override through, including one still in flight', async () => {
    const { mock } = show({
      layer: { vertical_width: 1300, vertical_origin: 900 },
    })
    await enter('vert. advance', '1200')
    await waitFor(() => expect(edits(mock)).toHaveLength(1))
    expect(edits(mock)[0]?.command).toMatchObject({
      SetVerticalMetrics: { advance: 1200, origin: 900 },
    })

    // The model has not caught up here — the fixture still reports the old
    // layer — and `SetVerticalMetrics` writes both fields. The second edit must
    // therefore carry the advance this panel *just sent*, not the stale 1300,
    // or typing an advance and then an origin would silently undo the advance.
    await enter('vert. origin', '920')
    await waitFor(() => expect(edits(mock)).toHaveLength(2))
    expect(edits(mock)[1]?.command).toMatchObject({
      SetVerticalMetrics: { advance: 1200, origin: 920 },
    })

    // A third edit keeps carrying it, and TSB — which owns the origin — leaves
    // the advance alone too.
    await enter('TSB', '40')
    await waitFor(() => expect(edits(mock)).toHaveLength(3))
    expect(edits(mock)[2]?.command).toMatchObject({
      SetVerticalMetrics: { advance: 1200 },
    })
  })

  it('clears both overrides in one command', async () => {
    const { mock } = show({ layer: { vertical_width: 0, vertical_origin: 0 } })
    await userEvent
      .setup()
      .click(
        screen.getByRole('button', { name: 'Use master vertical metrics' }),
      )
    await waitFor(() => expect(edits(mock)).toHaveLength(1))
    expect(edits(mock)[0]?.command).toMatchObject({
      SetVerticalMetrics: { advance: null, origin: null },
    })
  })

  it('computes BSB from the latest origin before its model reload arrives', async () => {
    const { mock } = show()
    await enter('vert. origin', '910')
    await enter('BSB', '250')
    await waitFor(() => expect(edits(mock)).toHaveLength(2))
    expect(edits(mock)[1]?.command).toMatchObject({
      SetVerticalMetrics: { advance: 1260, origin: 910 },
    })
  })

  it('does not reuse a rejected advance in a later origin edit', async () => {
    const { mock, errors } = show({
      apply: ({ command }) => {
        if (
          'SetVerticalMetrics' in command &&
          command.SetVerticalMetrics.advance === 70000
        )
          throw new Error('Vertical advance is out of range')
        return noChange([42])
      },
    })
    await enter('vert. advance', '70000')
    await waitFor(() => expect(errors).toHaveBeenCalledOnce())
    await enter('vert. origin', '910')
    await waitFor(() => expect(edits(mock)).toHaveLength(2))
    expect(edits(mock)[1]?.command).toMatchObject({
      SetVerticalMetrics: { advance: null, origin: 910 },
    })
    expect(errors).toHaveBeenCalledOnce()
  })

  it('keeps a successful advance when a rejected edit precedes a queued BSB edit', async () => {
    let rejectAdvance: (failure: Error) => void = () => undefined
    const pending = new Promise<never>((_, reject) => {
      rejectAdvance = reject
    })
    const { mock, errors } = show({
      apply: ({ command }) => {
        if (
          'SetVerticalMetrics' in command &&
          command.SetVerticalMetrics.advance === 70000
        )
          return pending
        return noChange([42])
      },
    })
    await enter('vert. advance', '1200')
    await enter('vert. advance', '70000')
    await waitFor(() => expect(edits(mock)).toHaveLength(2))
    await enter('TSB', '210')
    await enter('BSB', '250')
    expect(edits(mock)).toHaveLength(2)
    await act(async () => {
      rejectAdvance(new Error('Invalid advance'))
    })
    await waitFor(() => expect(edits(mock)).toHaveLength(4))
    expect(edits(mock)[2]?.command).toMatchObject({
      SetVerticalMetrics: { advance: 1200, origin: 910 },
    })
    expect(edits(mock)[3]?.command).toMatchObject({
      SetVerticalMetrics: { advance: 1260, origin: 910 },
    })
    expect(errors).toHaveBeenCalledOnce()
  })

  it('drops queued edits when the target changes before their preceding command finishes', async () => {
    let release: () => void = () => undefined
    const pending = new Promise<void>((resolve) => {
      release = resolve
    })
    const { mock, view, glyph, layer, scene, outline, reload } = show({
      apply: async () => {
        await pending
        return noChange([42])
      },
    })
    await enter('vert. advance', '1200')
    await waitFor(() => expect(edits(mock)).toHaveLength(1))
    await enter('vert. origin', '910')
    view.rerender(
      <VerticalMetricsPanel
        gid={42}
        target={{ doc: 2, glyph: glyph.name, layer: layer.id }}
        glyph={glyph}
        scene={scene}
        outline={outline}
        reload={reload}
      />,
    )
    await act(async () => {
      release()
    })
    expect(edits(mock)).toHaveLength(1)
    expect(reload).not.toHaveBeenCalled()
    await enter('vert. origin', '930')
    await waitFor(() => expect(edits(mock)).toHaveLength(2))
    expect(edits(mock)[1]).toMatchObject({
      doc: 2,
      command: { SetVerticalMetrics: { advance: null, origin: 930 } },
    })
  })

  it('disables sidebearing edits for empty ink while retaining advance and origin editing', () => {
    show({ bbox: null })
    expect(screen.getByRole('textbox', { name: 'TSB' })).toBeDisabled()
    expect(screen.getByRole('textbox', { name: 'BSB' })).toBeDisabled()
    expect(screen.getByRole('textbox', { name: 'vert. advance' })).toBeEnabled()
    expect(screen.getByRole('textbox', { name: 'vert. origin' })).toBeEnabled()
    expect(screen.getByText('Sidebearings require an outline.')).toBeVisible()
  })

  it('uses inherited origin after clearing an explicit override before reload', async () => {
    const { mock } = show({ layer: { vertical_origin: 900 } })
    await userEvent
      .setup()
      .click(
        screen.getByRole('button', { name: 'Use master vertical metrics' }),
      )
    await enter('BSB', '250')
    await waitFor(() => expect(edits(mock)).toHaveLength(2))
    expect(edits(mock)[1]?.command).toMatchObject({
      SetVerticalMetrics: { advance: 1230, origin: null },
    })
  })

  it('forgets acknowledged overrides before an external undo', async () => {
    const { mock, view, glyph, layer, scene, outline, reload, stored } = show()
    await enter('vert. origin', '910')
    await waitFor(() => expect(reload).toHaveBeenCalledOnce())
    const props = {
      target: { doc: 1, glyph: glyph.name, layer: layer.id },
      gid: 42,
      scene,
      outline,
      reload,
    }
    view.rerender(
      <VerticalMetricsPanel
        {...props}
        glyph={{ ...glyph, layers: [{ ...layer, vertical_origin: 910 }] }}
      />,
    )
    view.rerender(<VerticalMetricsPanel {...props} glyph={glyph} />)
    stored.set(1, glyph)
    await enter('BSB', '250')
    await waitFor(() => expect(edits(mock)).toHaveLength(2))
    expect(edits(mock)[1]?.command).toMatchObject({
      SetVerticalMetrics: { advance: 1230, origin: null },
    })
  })

  it('uses current kernel ink bounds for a sidebearing after other edits settle', async () => {
    const { mock } = show({
      readOutline: () =>
        makeLayerOutline(makeLayer(), { bbox: [40, -120, 560, 710] }),
    })
    await enter('TSB', '210')
    await enter('BSB', '250')
    await waitFor(() => expect(edits(mock)).toHaveLength(2))
    expect(edits(mock)[0]?.command).toMatchObject({
      SetVerticalMetrics: { origin: 920 },
    })
    expect(edits(mock)[1]?.command).toMatchObject({
      SetVerticalMetrics: { advance: 1290, origin: 920 },
    })
  })

  it('rejects invalid numeric input without applying a command', async () => {
    const { mock } = show()
    await enter('TSB', 'oops')
    expect(edits(mock)).toHaveLength(0)
    expect(screen.getByRole('textbox', { name: 'TSB' })).toHaveValue('180')
  })

  it('uses an external undo that arrives before the previous model reload', async () => {
    const { mock, reload, stored, glyph } = show()
    await enter('vert. origin', '910')
    await waitFor(() => expect(reload).toHaveBeenCalledOnce())
    stored.set(1, glyph)
    await enter('BSB', '250')
    await waitFor(() => expect(edits(mock)).toHaveLength(2))
    expect(edits(mock)[1]?.command).toMatchObject({
      SetVerticalMetrics: { advance: 1230, origin: null },
    })
  })

  it('reconciles a rejected value even when the model remains unchanged', async () => {
    const { mock, reload, errors } = show({ reject: true })
    await enter('BSB', '70000')
    await waitFor(() => expect(reload).toHaveBeenCalledOnce())
    expect(errors).toHaveBeenCalledOnce()
    expect(screen.getByRole('textbox', { name: 'BSB' })).toHaveValue('120')
    expect(edits(mock)).toHaveLength(1)
  })
})
