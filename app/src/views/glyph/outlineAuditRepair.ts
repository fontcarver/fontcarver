/** Bounded, token-scoped Fix All over the kernel's current geometry. */
import { call, type Contour } from '../../api'
import { editableContour } from '../../editor/editorMetadata'
import {
  documentEditRevision,
  holdDocumentEdits,
  noteDocumentEdit,
  pendingDocumentEdits,
} from '../../editor/editSession'
import type { LayerTarget } from '../../editor/pointCommands'

import {
  auditOutline,
  outlineAuditFix,
  type OutlineAuditSettings,
} from './outlineAuditModel'

export const MAX_AUDIT_REPAIR_PASSES = 32
const MAX_REPAIR_POINTS = 100_000

/** A refusal that the panel translates; transport failures retain their detail. */
export class OutlineAuditRepairError extends Error {
  constructor(readonly reason: 'stale' | 'limit') {
    super(reason)
  }
}

/** Bounded cycle key; a hash collision conservatively cancels, never accepts. */
function fingerprint(contours: readonly Contour[]): string {
  const serialized = JSON.stringify(contours)
  let hash = 2166136261
  for (let i = 0; i < serialized.length; i += 1)
    hash = Math.imul(hash ^ serialized.charCodeAt(i), 16777619)
  return `${serialized.length}:${hash}`
}

/**
 * Repairs enabled safe issues in one undo entry, refreshing indexes each pass.
 * Failures and non-convergence atomically cancel only our matching live token.
 * The published barrier keeps subsequent editor gestures behind cancellation.
 */
export function repairAllOutline({
  target,
  gid,
  original,
  settings,
  lockedPaths,
  label,
  isCurrent,
}: {
  target: LayerTarget
  gid: number
  original: readonly Contour[]
  settings: OutlineAuditSettings
  lockedPaths: ReadonlySet<string>
  label: string
  isCurrent: () => boolean
}): Promise<void> {
  const { doc } = target
  const prior = pendingDocumentEdits(doc)
  noteDocumentEdit(doc)
  const revision = documentEditRevision(doc)
  const checkCurrent = () => {
    if (!isCurrent() || documentEditRevision(doc) !== revision)
      throw new OutlineAuditRepairError('stale')
  }
  // Deferring the work publishes its barrier before even the first API read.
  const running = Promise.resolve().then(async () => {
    await prior
    checkCurrent()
    const read = () => call('glyphs.outline', { doc, gid, layer: target.layer })
    let fresh = await read()
    checkCurrent()
    if (JSON.stringify(fresh.contours) !== JSON.stringify(original))
      throw new OutlineAuditRepairError('stale')
    const token = await call('doc.edit_begin', { doc, label })
    try {
      const seen = new Set<string>()
      for (let pass = 0; ; pass += 1) {
        checkCurrent()
        if (
          fresh.contours.reduce(
            (count, contour) => count + contour.points.length,
            0,
          ) > MAX_REPAIR_POINTS
        )
          throw new OutlineAuditRepairError('limit')
        const issues = auditOutline(fresh.contours, settings).issues.filter(
          (issue) =>
            issue.fix &&
            issue.points.every((point) =>
              editableContour(fresh.contours, point.contour, lockedPaths, null),
            ),
        )
        const command = outlineAuditFix(target, fresh.contours, issues)
        if (!command) break
        const key = fingerprint(fresh.contours)
        if (pass >= MAX_AUDIT_REPAIR_PASSES || seen.has(key))
          throw new OutlineAuditRepairError('limit')
        seen.add(key)
        await call('doc.edit_apply', { doc, token, command })
        checkCurrent()
        fresh = await read()
      }
      checkCurrent()
      await call('doc.edit_end', { doc, token })
    } catch (error) {
      // No global undo fallback: a stale token may belong to an already closed
      // gesture, and undo would then remove somebody else's newer operation.
      await call('doc.edit_cancel', { doc, token })
      throw error
    }
  })
  void holdDocumentEdits(doc, running)
  return running
}
