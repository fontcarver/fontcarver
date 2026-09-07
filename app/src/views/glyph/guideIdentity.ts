import type { DocId, LayerId, MasterId } from '../../api'
import type { GuideScope } from '../../editor/selection'

/** Document and layer/master ownership of guides shown on one canvas. */
export interface GuideOwner {
  doc: DocId
  layer: LayerId
  master: MasterId | null
}

/** Session identity of one measurement guide. */
export function measurementGuideKey(
  owner: GuideOwner,
  scope: GuideScope,
  index: number,
): string {
  const container = scope === 'layer' ? owner.layer : (owner.master ?? '')
  return JSON.stringify([owner.doc, scope, container, index])
}
