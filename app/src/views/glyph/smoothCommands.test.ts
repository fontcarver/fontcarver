import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { setTransport, type ApplyParams, type Contour } from '../../api'
import { selectionOf, type EditTarget } from '../../editor/selection'
import {
  getCommand,
  isCommandEnabled,
  resetCommands,
  runCommand,
  setCommandErrorReporter,
} from '../../shell/commands'
import { initialEditorState, useEditorStore } from '../../stores/editorStore'
import {
  makeCell,
  makeContour,
  makeGlyph,
  makeGlyphDetail,
  makeLayer,
  makeLayerOutline,
  makeMaster,
  makePoint,
} from '../../test/fontFixtures'
import {
  createMockTransport,
  noChange,
  type MockTransport,
} from '../../test/mockTransport'

import { buildScene } from './glyphScene'
import { registerSmoothCommands } from './smoothCommands'

function cubicContour(closed = false): Contour {
  return makeContour(
    [
      makePoint(-100, 100, closed ? 'Curve' : 'Move'),
      makePoint(-70, 50, 'OffCurve'),
      makePoint(-30, 0, 'OffCurve'),
      makePoint(0, 0, 'Curve'),
      makePoint(60, 0, 'OffCurve'),
      makePoint(70, 100, 'OffCurve'),
      makePoint(100, 100, 'Curve'),
      ...(closed
        ? [makePoint(70, 150, 'OffCurve'), makePoint(-70, 150, 'OffCurve')]
        : []),
    ],
    closed,
  )
}

const reload = vi.fn()

function publish(contours = [cubicContour()]): void {
  const layer = makeLayer({ contours })
  const glyph = makeGlyph({ layers: [layer] })
  useEditorStore.setState({
    ...initialEditorState(),
    activeGlyph: {
      doc: 1,
      gid: 42,
      glyph,
      scene: buildScene({
        detail: makeGlyphDetail(glyph),
        cell: makeCell(),
        master: makeMaster(),
        masterId: 'm1',
        unitsPerEm: 1000,
      }),
      cell: makeCell(),
      outline: makeLayerOutline(layer),
      master: 'm1',
      masterGuides: [],
      reload,
    },
  })
}

function select(...targets: EditTarget[]): void {
  useEditorStore.getState().setSelection(selectionOf(targets))
}

function transport(): MockTransport {
  const mock = createMockTransport({ 'doc.apply': () => noChange([42]) })
  setTransport(mock)
  return mock
}

function applied(mock: MockTransport): ApplyParams | undefined {
  return mock.calls.find((entry) => entry.method === 'doc.apply')?.params as
    ApplyParams | undefined
}

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0))
  await new Promise((resolve) => setTimeout(resolve, 0))
}

beforeEach(() => {
  reload.mockClear()
  resetCommands()
  useEditorStore.setState(initialEditorState())
  registerSmoothCommands()
})

afterEach(() => {
  resetCommands()
  setCommandErrorReporter(() => undefined)
  setTransport(undefined)
})

describe('G2 smoothing menu', () => {
  it('registers the actions in the Path menu and requires an edited layer', () => {
    for (const id of ['path.superSmooth', 'path.harmonize']) {
      expect(getCommand(id)).toMatchObject({
        menu: 'path',
        disabledReason: 'cmd.reason.noCubicJoins',
      })
      expect(isCommandEnabled(id)).toBe(false)
      expect(runCommand(id)).toBe(false)
    }
  })

  it.each([
    [],
    [{ kind: 'node', contour: 0, point: 0 }],
    [{ kind: 'node', contour: 0, point: 6 }],
    [{ kind: 'handle', contour: 0, point: 2 }],
    [{ kind: 'node', contour: 0, point: 2 }],
    [{ kind: 'node', contour: 99, point: 3 }],
    [
      { kind: 'node', contour: 0, point: 3 },
      { kind: 'component', index: 0 },
    ],
  ] satisfies EditTarget[][])(
    'rejects unsupported selection %j',
    (...targets) => {
      const mock = transport()
      publish()
      select(...targets)
      expect(runCommand('path.superSmooth')).toBe(false)
      expect(runCommand('path.harmonize')).toBe(false)
      expect(mock.calls).toEqual([])
    },
  )

  it('rejects line or quadratic joins', () => {
    for (const kind of ['Line', 'QCurve'] as const) {
      const contour = cubicContour()
      contour.points[6]!.kind = kind
      publish([contour])
      select({ kind: 'node', contour: 0, point: 3 })
      expect(isCommandEnabled('path.superSmooth')).toBe(false)
      expect(isCommandEnabled('path.harmonize')).toBe(false)
    }
  })

  it('enables selected joins in stored order as one undo command', async () => {
    const mock = transport()
    publish([cubicContour(), cubicContour()])
    select(
      { kind: 'node', contour: 1, point: 3 },
      { kind: 'node', contour: 0, point: 3 },
    )
    expect(getCommand('path.superSmooth')?.detail?.()).toBe('Disabled')
    expect(runCommand('path.superSmooth')).toBe(true)
    await settle()
    expect(applied(mock)).toEqual({
      doc: 1,
      label: 'Enable super-smooth nodes',
      command: {
        SetSuperSmooth: {
          glyph: 'a',
          layer: 'layer-1',
          points: [
            { contour: 0, point: 3 },
            { contour: 1, point: 3 },
          ],
          enabled: true,
        },
      },
    })
    expect(reload).toHaveBeenCalledOnce()
  })

  it('disables marked nodes even when topology has changed', async () => {
    const mock = transport()
    const contour = cubicContour()
    contour.points[3] = { ...contour.points[3]!, super_smooth: true }
    contour.points[6]!.kind = 'Line'
    publish([contour])
    select({ kind: 'node', contour: 0, point: 3 })
    expect(getCommand('path.superSmooth')?.detail?.()).toBe('Enabled')
    expect(isCommandEnabled('path.harmonize')).toBe(false)
    expect(runCommand('path.superSmooth')).toBe(true)
    await settle()
    expect(applied(mock)).toMatchObject({
      label: 'Disable super-smooth nodes',
      command: { SetSuperSmooth: { enabled: false } },
    })
  })

  it('enables all selected nodes when their persistent state is mixed', async () => {
    const mock = transport()
    const contour = cubicContour(true)
    contour.points[0] = { ...contour.points[0]!, super_smooth: true }
    publish([contour])
    select(
      { kind: 'node', contour: 0, point: 3 },
      { kind: 'node', contour: 0, point: 0 },
    )
    expect(getCommand('path.superSmooth')?.detail?.()).toBe('Mixed')
    expect(runCommand('path.superSmooth')).toBe(true)
    await settle()
    expect(applied(mock)?.command).toMatchObject({
      SetSuperSmooth: {
        points: [
          { contour: 0, point: 0 },
          { contour: 0, point: 3 },
        ],
        enabled: true,
      },
    })
  })

  it('harmonizes a closed seam without changing persistent state', async () => {
    const mock = transport()
    publish([cubicContour(true)])
    select({ kind: 'node', contour: 0, point: 0 })
    expect(runCommand('path.harmonize')).toBe(true)
    await settle()
    expect(applied(mock)).toEqual({
      doc: 1,
      label: 'Harmonize nodes',
      command: {
        HarmonizeNodes: {
          glyph: 'a',
          layer: 'layer-1',
          points: [{ contour: 0, point: 0 }],
        },
      },
    })
  })

  it('reports geometric rejection and reconciles the edited layer', async () => {
    const error = new Error('Selected join has incompatible curvature')
    const report = vi.fn()
    setCommandErrorReporter(report)
    setTransport(
      createMockTransport({
        'doc.apply': () => {
          throw error
        },
      }),
    )
    publish()
    select({ kind: 'node', contour: 0, point: 3 })
    runCommand('path.harmonize')
    await settle()
    expect(report).toHaveBeenCalledWith(error)
    expect(reload).toHaveBeenCalledOnce()
  })
})
