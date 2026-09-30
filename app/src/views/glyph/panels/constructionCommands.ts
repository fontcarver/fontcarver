/** Typed, atomic construction edits shared by the inspector and its tests. */
import type { Command, Layer, PointRef } from '../../../api'
import type { LayerTarget } from '../../../editor/pointCommands'
import { servantNeighbors, type DetectedStem } from './constructionGeometry'
import { addGuideCommand, newGuide } from './panelCommands'

/** Make one selected node an X/Y interpolation of its non-servant neighbors. */
export function setServantCommand(
  target: LayerTarget,
  layer: Layer,
  node: PointRef,
  axis: 'X' | 'Y',
): Command | null {
  const constrained = new Set(
    (layer.node_links ?? [])
      .filter((link) => link.axis === axis)
      .map((link) => link.node.id),
  )
  const servants = new Set<string>()
  layer.contours.forEach((contour, ci) =>
    contour.points.forEach((point, pi) => {
      if (point.id && constrained.has(point.id)) servants.add(`${ci}:${pi}`)
    }),
  )
  const neighbors = servantNeighbors(layer.contours, node, servants)
  if (!neighbors) return null
  const [first, second] = neighbors
  const a = layer.contours[first.contour]?.points[first.point]
  const b = layer.contours[second.contour]?.points[second.point]
  const coordinate = axis === 'X' ? 'x' : 'y'
  if (!a || !b || Math.abs(a[coordinate] - b[coordinate]) < 1e-9) return null
  return {
    NodeLinks: {
      SetServant: {
        glyph: target.glyph,
        layer: target.layer,
        node,
        first,
        second,
        axis,
      },
    },
  }
}

/** Link selected nodes to a layer guide, optionally first projecting onto it. */
export function magneticLinkCommand(
  target: LayerTarget,
  guideIndex: number,
  points: readonly PointRef[],
  project: boolean,
): Command {
  const params = {
    glyph: target.glyph,
    layer: target.layer,
    guide_index: guideIndex,
    points: [...points],
  }
  const link: Command = { NodeLinks: { LinkToGuide: params } }
  return project
    ? { Batch: [{ NodeLinks: { ProjectToGuide: params } }, link] }
    : link
}

/** Add centerline guides whose attached edge nodes retain the detected stem width. */
export function stemGuidesCommand(
  target: LayerTarget,
  layer: Layer,
  stems: readonly DetectedStem[],
  name: (width: number) => string,
): Command | null {
  const commands: Command[] = []
  let guideIndex = layer.guides.length
  const seen = new Set<string>()
  for (const stem of stems) {
    const position = (stem.low.position + stem.high.position) / 2
    const key = `${stem.axis}:${position}`
    if (seen.has(key)) continue
    seen.add(key)
    const angle = stem.axis === 'X' ? 90 : 0
    const matching = layer.guides.findIndex(
      (guide) =>
        Math.abs(guide.angle - angle) < 1e-7 &&
        Math.abs((stem.axis === 'X' ? guide.x : guide.y) - position) < 1e-7,
    )
    const index = matching >= 0 ? matching : guideIndex++
    const points = stems
      .filter(
        (candidate) =>
          candidate.axis === stem.axis &&
          Math.abs(
            (candidate.low.position + candidate.high.position) / 2 - position,
          ) < 1e-7,
      )
      .flatMap((candidate) => [
        ...candidate.low.points,
        ...candidate.high.points,
      ])
      .filter(
        (point, i, refs) =>
          refs.findIndex(
            (other) =>
              other.contour === point.contour && other.point === point.point,
          ) === i,
      )
    if (matching < 0)
      commands.push(
        addGuideCommand(target, 'layer', null, {
          ...newGuide(
            stem.axis === 'X' ? position : 0,
            stem.axis === 'Y' ? position : 0,
            angle,
          ),
          name: name(stem.width),
        }),
      )
    commands.push(magneticLinkCommand(target, index, points, false))
  }
  return commands.length ? { Batch: commands } : null
}
