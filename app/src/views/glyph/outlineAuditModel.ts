/**
 * The always-on outline audit of the editable source layer (audit row 35).
 *
 * The audit reads the *stored* points, never a flattened path, so every issue
 * it reports can be addressed by a command that names the same indexes. Each
 * issue carries the repair it would apply, which is what makes Fix and Fix All
 * honest: the panel offers a fix exactly when there is one, and the repair is
 * re-derived from freshly fetched contours before it is sent, so an outline
 * that moved under the user cannot be "repaired" from a stale reading.
 */

import type { Command, Contour, PointRef } from '../../api'
import {
  buildEditOutline,
  extremumCandidates,
  segmentPoint,
  type EditSegment,
} from '../../editor/outlineGeometry'
import type { LayerTarget } from '../../editor/pointCommands'

/** Independent checks available in the active source-layer audit. */
export const OUTLINE_AUDIT_TESTS = [
  'open',
  'fractional',
  'coincident',
  'extrema',
] as const
export type OutlineAuditTest = (typeof OUTLINE_AUDIT_TESTS)[number]
export type OutlineAuditSettings = Record<OutlineAuditTest, boolean>

/** The audit is enabled until the user disables an individual check. */
export const DEFAULT_OUTLINE_AUDIT: OutlineAuditSettings = {
  open: true,
  fractional: true,
  coincident: true,
  extrema: true,
}

/** A standalone all-checks setting, independent of the mutable defaults. */
export const ALL_OUTLINE_AUDIT_TESTS: OutlineAuditSettings = {
  ...DEFAULT_OUTLINE_AUDIT,
}

/**
 * How far an extremum has to stand out from its segment's endpoints, in font
 * units, before the audit calls it missing.
 *
 * The kernel's own `add_extrema` treats anything within 0.01 units of an
 * endpoint as already present; an audit marker needs a wider margin than a
 * geometry operation, because a node a third of a unit off an extremum is a
 * rounding artefact and marking it would bury the real defects.
 */
export const EXTREMUM_TOLERANCE = 0.5

/** The repair one issue would apply. */
export type OutlineAuditFix =
  /** Round one point to the integer grid. */
  | { kind: 'round'; point: PointRef; x: number; y: number }
  /** Close an open contour by joining its two ends. */
  | { kind: 'close'; contour: number; first: number; last: number }
  /** Delete the duplicates of a run of coincident nodes, keeping the first. */
  | { kind: 'dedupe'; contour: number; points: PointRef[] }
  /** Insert an on-curve node at a missing extremum. */
  | {
      kind: 'insertExtremum'
      contour: number
      segment: number
      t: number
      x: number
      y: number
    }

/**
 * Which repairs may share one undo entry per contour.
 *
 * A structural repair renumbers stored points or segments, so two of different
 * kinds in the same contour cannot be described by indexes taken before either
 * ran. Each pass therefore applies the highest-priority kind present in each
 * contour. Fix All re-audits the kernel between passes in one gesture, so the
 * next pass always uses current indexes. Contours are independent.
 */
const FIX_PRIORITY: readonly OutlineAuditFix['kind'][] = [
  'dedupe',
  'insertExtremum',
  'close',
  'round',
]

/** Rank of a repair kind, for a total order over the chosen repairs. */
function kindRank(kind: OutlineAuditFix['kind']): number {
  return FIX_PRIORITY.indexOf(kind)
}

/** A finite font-space marker and the stored points it describes. */
export interface OutlineAuditIssue {
  id: string
  test: OutlineAuditTest
  x: number
  y: number
  points: PointRef[]
  /** The repair this issue offers, or absent when it needs manual review. */
  fix?: OutlineAuditFix
}

/** Reports are bounded to keep malformed or extremely large outlines usable. */
export interface OutlineAuditReport {
  issues: OutlineAuditIssue[]
  truncated: boolean
}

/** Whether a stored point can carry a marker at all. */
function finitePoint(point: { x: number; y: number } | undefined): boolean {
  return (
    point !== undefined && Number.isFinite(point.x) && Number.isFinite(point.y)
  )
}

/** The on-curve stored indexes of a contour, in order. */
function onCurveIndexes(contour: Contour): number[] {
  const found: number[] = []
  contour.points.forEach((point, index) => {
    if (point.kind !== 'OffCurve') found.push(index)
  })
  return found
}

/**
 * Whether the listed indexes form one cyclically adjacent run of on-curve
 * points, which is the only coincidence the audit is willing to repair: two
 * separate contours meeting at a point, or a node coinciding with a handle,
 * are ordinary drawing, not a duplicate node.
 */
function adjacentRun(contour: Contour, points: readonly number[]): boolean {
  const order = onCurveIndexes(contour)
  if (points.length < 2 || points.length > order.length) return false
  const positions = points
    .map((index) => order.indexOf(index))
    .sort((a, b) => a - b)
  if (positions.some((position) => position < 0)) return false
  const contiguous = positions.every(
    (position, offset) =>
      offset === 0 || position === (positions[offset - 1] as number) + 1,
  )
  if (contiguous) return true
  if (!contour.closed) return false
  // A run across the seam: the low block ends the list and the high one starts it.
  const gap = positions.findIndex(
    (position, offset) =>
      offset > 0 && position !== (positions[offset - 1] as number) + 1,
  )
  if (gap < 0) return false
  const head = positions.slice(0, gap)
  const tail = positions.slice(gap)
  return (
    head[0] === 0 &&
    head.every((position, offset) => position === offset) &&
    (tail.at(-1) as number) === order.length - 1 &&
    tail.every(
      (position, offset) =>
        offset === 0 || position === (tail[offset - 1] as number) + 1,
    )
  )
}

/** Extrema of one segment that no node sits on. */
function missingExtrema(
  segment: EditSegment,
): { t: number; x: number; y: number }[] {
  if (segment.kind === 'line') return []
  const found: { t: number; x: number; y: number }[] = []
  for (const t of extremumCandidates(segment)) {
    const at = segmentPoint(segment, t)
    if (!finitePoint(at)) continue
    const standsOut = (['x', 'y'] as const).some(
      (axis) =>
        Math.abs(at[axis] - segment.from[axis]) > EXTREMUM_TOLERANCE &&
        Math.abs(at[axis] - segment.to[axis]) > EXTREMUM_TOLERANCE,
    )
    if (standsOut) found.push({ t, x: at.x, y: at.y })
  }
  return found
}

/** Inspect stored source points without mutating, normalizing or flattening them. */
export function auditOutline(
  contours: readonly Contour[],
  settings: OutlineAuditSettings = DEFAULT_OUTLINE_AUDIT,
): OutlineAuditReport {
  const report: OutlineAuditReport = { issues: [], truncated: false }
  const coincident = new Map<
    string,
    { issue: OutlineAuditIssue; contour: number }
  >()
  let inspected = 0
  const add = (issue: OutlineAuditIssue) => {
    if (report.issues.length < 100) report.issues.push(issue)
    else report.truncated = true
  }
  outer: for (let contour = 0; contour < contours.length; contour += 1) {
    const entry = contours[contour]
    if (!entry) continue
    const first = entry.points.findIndex((point) => finitePoint(point))
    const start = entry.points[first]
    if (settings.open && !entry.closed && start) {
      const last = entry.points.length - 1
      const closable =
        first === 0 &&
        last > 0 &&
        entry.points[0]?.kind !== 'OffCurve' &&
        entry.points[last]?.kind !== 'OffCurve' &&
        finitePoint(entry.points[last])
      add({
        id: `open:${contour}`,
        test: 'open',
        x: start.x,
        y: start.y,
        points: [{ contour, point: first }],
        ...(closable
          ? { fix: { kind: 'close' as const, contour, first: 0, last } }
          : {}),
      })
    }
    for (let point = 0; point < entry.points.length; point += 1) {
      inspected += 1
      if (inspected > 100_000) {
        report.truncated = true
        break outer
      }
      const at = entry.points[point]
      if (!finitePoint(at) || !at) continue
      const ref = { contour, point }
      if (
        settings.fractional &&
        (!Number.isInteger(at.x) || !Number.isInteger(at.y))
      ) {
        add({
          id: `fractional:${contour}:${point}`,
          test: 'fractional',
          x: at.x,
          y: at.y,
          points: [ref],
          fix: {
            kind: 'round',
            point: ref,
            x: Math.round(at.x),
            y: Math.round(at.y),
          },
        })
      }
      if (!settings.coincident) continue
      const key = `${at.x}:${at.y}`
      const previous = coincident.get(key)
      if (previous) {
        previous.issue.points.push(ref)
        if (previous.contour !== contour) previous.contour = -1
      } else
        coincident.set(key, {
          contour,
          issue: {
            id: `coincident:${key}`,
            test: 'coincident',
            x: at.x,
            y: at.y,
            points: [ref],
          },
        })
    }
  }
  for (const { issue, contour } of coincident.values()) {
    if (issue.points.length < 2) continue
    const entry = contour >= 0 ? contours[contour] : undefined
    const indexes = issue.points.map((point) => point.point)
    if (
      entry &&
      indexes.every((index) => entry.points[index]?.kind !== 'OffCurve') &&
      adjacentRun(entry, indexes)
    )
      issue.fix = {
        kind: 'dedupe',
        contour,
        points: issue.points.slice(1),
      }
    add(issue)
  }
  if (settings.extrema) {
    const outline = buildEditOutline(contours)
    for (const segment of outline.segments)
      for (const found of missingExtrema(segment)) {
        add({
          id: `extrema:${segment.contour}:${segment.index}:${found.t}`,
          test: 'extrema',
          x: found.x,
          y: found.y,
          points: [segment.fromRef ?? { contour: segment.contour, point: 0 }],
          fix: {
            kind: 'insertExtremum',
            contour: segment.contour,
            segment: segment.index,
            t: found.t,
            x: found.x,
            y: found.y,
          },
        })
        if (report.truncated) break
      }
  }
  return report
}

/** Whether two repairs describe exactly the same edit. */
function sameFix(a: OutlineAuditFix, b: OutlineAuditFix | undefined): boolean {
  return b !== undefined && JSON.stringify(a) === JSON.stringify(b)
}

/** The contour a repair renumbers. */
function fixContour(fix: OutlineAuditFix): number {
  return fix.kind === 'round' ? fix.point.contour : fix.contour
}

/** The commands one repair applies, in the order they must be sent. */
function commandsFor(target: LayerTarget, fix: OutlineAuditFix): Command[] {
  const { glyph, layer } = target
  switch (fix.kind) {
    case 'round':
      return [
        {
          SetPointPosition: {
            glyph,
            layer,
            point: fix.point,
            x: fix.x,
            y: fix.y,
          },
        },
      ]
    case 'close':
      return [
        {
          JoinContours: {
            glyph,
            layer,
            a: { contour: fix.contour, point: fix.last },
            b: { contour: fix.contour, point: fix.first },
          },
        },
      ]
    case 'dedupe':
      // Emitted by `outlineAuditFix`, which merges every run into one command.
      return []
    case 'insertExtremum':
      return [
        {
          InsertPointOnSegment: {
            glyph,
            layer,
            contour: fix.contour,
            segment: fix.segment,
            t: fix.t,
          },
        },
      ]
  }
}

/**
 * One undo entry repairing the reviewed issues, or `null` when none applies.
 *
 * `contours` are the contours as the kernel has them *now*: the audit is run
 * again over them and only an issue whose repair is still exactly the one the
 * user reviewed is applied, so a concurrent edit cannot be repaired by proxy.
 */
export function outlineAuditFix(
  target: LayerTarget,
  contours: readonly Contour[],
  issues: readonly OutlineAuditIssue[],
): Command | null {
  const requested = new Set(issues.map((issue) => issue.test))
  const fresh = auditOutline(contours, {
    open: requested.has('open'),
    fractional: requested.has('fractional'),
    coincident: requested.has('coincident'),
    extrema: requested.has('extrema'),
  })
  const current = new Map(fresh.issues.map((issue) => [issue.id, issue]))
  const chosen: OutlineAuditFix[] = []
  const seen = new Set<string>()
  for (const issue of issues) {
    const live = current.get(issue.id)
    if (!live?.fix || !issue.fix || !sameFix(live.fix, issue.fix)) continue
    if (seen.has(issue.id)) continue
    seen.add(issue.id)
    chosen.push(live.fix)
  }
  // One kind of repair per contour; see FIX_PRIORITY.
  const kindOf = new Map<number, OutlineAuditFix['kind']>()
  for (const fix of chosen) {
    const contour = fixContour(fix)
    const held = kindOf.get(contour)
    if (
      held === undefined ||
      FIX_PRIORITY.indexOf(fix.kind) < FIX_PRIORITY.indexOf(held)
    )
      kindOf.set(contour, fix.kind)
  }
  const applied = chosen.filter(
    (fix) => kindOf.get(fixContour(fix)) === fix.kind,
  )
  // A total order: kind first, then descending segment so an insertion never
  // renumbers a segment a later command in the same batch still names.
  applied.sort(
    (a, b) =>
      kindRank(a.kind) - kindRank(b.kind) ||
      (a.kind === 'insertExtremum' && b.kind === 'insertExtremum'
        ? b.segment - a.segment || b.t - a.t
        : 0),
  )
  // Every duplicate goes in one `DeletePoints`: deleting can empty a contour
  // and renumber the list, so a second delete command could not be trusted.
  const duplicates = applied.flatMap((fix) =>
    fix.kind === 'dedupe' ? fix.points : [],
  )
  // Descending splits leave the original segment index naming its left piece.
  // Each following t is relative to that piece, whose endpoint was the prior t.
  const upperParameter = new Map<string, number>()
  const structural = applied.flatMap((fix) => {
    if (fix.kind !== 'insertExtremum') return commandsFor(target, fix)
    const key = `${fix.contour}:${fix.segment}`
    const upper = upperParameter.get(key) ?? 1
    upperParameter.set(key, fix.t)
    return commandsFor(target, { ...fix, t: fix.t / upper })
  })
  const commands = [
    ...(duplicates.length > 0
      ? [
          {
            DeletePoints: {
              glyph: target.glyph,
              layer: target.layer,
              points: duplicates,
              mode: 'KeepShape' as const,
            },
          } satisfies Command,
        ]
      : []),
    ...structural,
  ]
  if (commands.length === 0) return null
  return commands.length === 1 ? (commands[0] as Command) : { Batch: commands }
}
