/** Selection-scoped, undoable G2 smoothing actions for the Path menu. */

import type { Contour, PointRef } from '../../api'
import { selectedNodeRefs } from '../../editor/selection'
import { i18n } from '../../i18n'
import { registerCommands } from '../../shell/commands'

import { commitField } from './commitField'
import { pathContext, type PathContext } from './pathCommands'

interface SmoothingContext extends PathContext {
  points: PointRef[]
  allMarked: boolean
  anyMarked: boolean
  cubicJoins: boolean
}

/** Coarse topology check; the kernel validates curvature and degeneracy. */
function isCubicJoin(contour: Contour, index: number): boolean {
  const { points, closed } = contour
  if (points.length < 6 || points[index]?.kind !== 'Curve') return false
  if (!closed && (index < 3 || index + 3 >= points.length)) return false
  const at = (offset: number) =>
    points[(index + offset + points.length) % points.length]
  return (
    [-2, -1, 1, 2].every((offset) => at(offset)?.kind === 'OffCurve') &&
    at(-3)?.kind !== 'OffCurve' &&
    at(3)?.kind === 'Curve'
  )
}

function smoothingContext(): SmoothingContext | null {
  const context = pathContext()
  if (!context) return null
  const points = selectedNodeRefs(context.selection)
  if (points.length === 0 || points.length !== context.selection.size)
    return null
  const nodes = points.map(
    (ref) => context.contours[ref.contour]?.points[ref.point],
  )
  if (nodes.some((node) => !node || node.kind === 'OffCurve')) return null
  return {
    ...context,
    points,
    allMarked: nodes.every((node) => node?.super_smooth === true),
    anyMarked: nodes.some((node) => node?.super_smooth === true),
    cubicJoins: points.every((ref) => {
      const contour = context.contours[ref.contour]
      return contour !== undefined && isCubicJoin(contour, ref.point)
    }),
  }
}

/** Registers persistent super-smooth and one-shot harmonization commands. */
export function registerSmoothCommands(): void {
  registerCommands([
    {
      id: 'path.superSmooth',
      title: 'cmd.path.superSmooth',
      menu: 'path',
      group: 'nodes',
      disabledReason: 'cmd.reason.noCubicJoins',
      when: () => {
        const context = smoothingContext()
        return context !== null && (context.allMarked || context.cubicJoins)
      },
      detail: () => {
        const context = smoothingContext()
        if (!context) return undefined
        if (context.allMarked) return i18n.t('cmd.path.superSmoothEnabled')
        if (context.anyMarked) return i18n.t('cmd.path.superSmoothMixed')
        return i18n.t('cmd.path.superSmoothDisabled')
      },
      run: () => {
        const context = smoothingContext()
        if (!context || (!context.allMarked && !context.cubicJoins)) return
        const enabled = !context.allMarked
        commitField(
          context.target,
          {
            SetSuperSmooth: {
              glyph: context.target.glyph,
              layer: context.target.layer,
              points: context.points,
              enabled,
            },
          },
          i18n.t(
            enabled
              ? 'editor.undo.enableSuperSmooth'
              : 'editor.undo.disableSuperSmooth',
          ),
          context.reload,
        )
      },
    },
    {
      id: 'path.harmonize',
      title: 'cmd.path.harmonize',
      menu: 'path',
      group: 'nodes',
      disabledReason: 'cmd.reason.noCubicJoins',
      when: () => smoothingContext()?.cubicJoins === true,
      run: () => {
        const context = smoothingContext()
        if (!context?.cubicJoins) return
        commitField(
          context.target,
          {
            HarmonizeNodes: {
              glyph: context.target.glyph,
              layer: context.target.layer,
              points: context.points,
            },
          },
          i18n.t('editor.undo.harmonizeNodes'),
          context.reload,
        )
      },
    },
  ])
}
