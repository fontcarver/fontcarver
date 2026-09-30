import type { Command, Contour, PointRef } from '../../api'
import type { LayerTarget } from '../../editor/pointCommands'

/** Independent checks available in the active source-layer audit. */
export const OUTLINE_AUDIT_TESTS = ['open', 'fractional', 'coincident'] as const
export type OutlineAuditTest = (typeof OUTLINE_AUDIT_TESTS)[number]
export type OutlineAuditSettings = Record<OutlineAuditTest, boolean>

/** The audit is enabled until the user disables an individual check. */
export const DEFAULT_OUTLINE_AUDIT: OutlineAuditSettings = {
  open: true,
  fractional: true,
  coincident: true,
}

/** A finite font-space marker and the stored points it describes. */
export interface OutlineAuditIssue {
  id: string
  test: OutlineAuditTest
  x: number
  y: number
  points: PointRef[]
}

/** Reports are bounded to keep malformed or extremely large outlines usable. */
export interface OutlineAuditReport {
  issues: OutlineAuditIssue[]
  truncated: boolean
}

/** Inspect stored source points without mutating, normalizing or flattening them. */
export function auditOutline(
  contours: readonly Contour[],
  settings: OutlineAuditSettings = DEFAULT_OUTLINE_AUDIT,
): OutlineAuditReport {
  const report: OutlineAuditReport = { issues: [], truncated: false }
  const seen = new Map<string, OutlineAuditIssue>()
  let inspected = 0
  const add = (issue: OutlineAuditIssue) => {
    if (report.issues.length < 100) report.issues.push(issue)
    else report.truncated = true
  }
  outer: for (let contour = 0; contour < contours.length; contour += 1) {
    const entry = contours[contour]
    if (!entry) continue
    const first = entry.points.findIndex(
      (point) => Number.isFinite(point.x) && Number.isFinite(point.y),
    )
    const start = entry.points[first]
    if (settings.open && !entry.closed && start) {
      add({
        id: `open:${contour}`,
        test: 'open',
        x: start.x,
        y: start.y,
        points: [{ contour, point: first }],
      })
    }
    for (let point = 0; point < entry.points.length; point += 1) {
      inspected += 1
      if (inspected > 100_000) {
        report.truncated = true
        break outer
      }
      const at = entry.points[point]
      if (!at || !Number.isFinite(at.x) || !Number.isFinite(at.y)) continue
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
        })
      }
      if (!settings.coincident) continue
      const key = `${at.x}:${at.y}`
      const previous = seen.get(key)
      if (previous) previous.points.push(ref)
      else
        seen.set(key, {
          id: `coincident:${key}`,
          test: 'coincident',
          x: at.x,
          y: at.y,
          points: [ref],
        })
    }
  }
  for (const issue of seen.values()) if (issue.points.length > 1) add(issue)
  return report
}

/** Round only the reviewed fractional points, preserving topology and metadata. */
export function outlineAuditFix(
  target: LayerTarget,
  contours: readonly Contour[],
  issues: readonly OutlineAuditIssue[],
): Command | null {
  const commands: Command[] = []
  const fixed = new Set<string>()
  for (const issue of issues) {
    if (issue.test !== 'fractional') continue
    const ref = issue.points[0]
    const point = ref && contours[ref.contour]?.points[ref.point]
    if (!ref || !point || point.x !== issue.x || point.y !== issue.y) continue
    if (!Number.isFinite(point.x) || !Number.isFinite(point.y)) continue
    if (Number.isInteger(point.x) && Number.isInteger(point.y)) continue
    const key = `${ref.contour}:${ref.point}`
    if (fixed.has(key)) continue
    fixed.add(key)
    commands.push({
      SetPointPosition: {
        glyph: target.glyph,
        layer: target.layer,
        point: ref,
        x: Math.round(point.x),
        y: Math.round(point.y),
      },
    })
  }
  if (commands.length === 0) return null
  return commands.length === 1 ? (commands[0] ?? null) : { Batch: commands }
}
