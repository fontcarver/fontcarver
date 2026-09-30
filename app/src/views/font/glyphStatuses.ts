import {
  CircleOff,
  FileWarning,
  Info,
  LockKeyhole,
  MessageSquare,
  Ruler,
  type LucideIcon,
} from 'lucide-react'

import type { GlyphCell } from '../../api'

/** One status shown in a glyph's thumbnail and accessible description. */
export interface GlyphStatus {
  id: string
  labelKey: string
  Icon: LucideIcon
  tone: 'warning' | 'neutral'
}

const STATUSES = [
  ['metrics_out_of_sync', 'metricsOutOfSync', Ruler, 'warning'],
  ['incompatible_masters', 'incompatibleMasters', FileWarning, 'warning'],
  ['has_annotations', 'annotations', MessageSquare, 'neutral'],
  ['has_custom_glyph_info', 'customGlyphInfo', Info, 'neutral'],
  ['locked', 'locked', LockKeyhole, 'neutral'],
] as const

/** Status metadata is supplied by the kernel; thumbnails never infer it. */
export function glyphStatuses(cell: GlyphCell): GlyphStatus[] {
  const statuses: GlyphStatus[] = []
  for (const [id, label, Icon, tone] of STATUSES) {
    if (cell[id]) {
      statuses.push({
        id,
        labelKey: `fontView.glyphStatus.${label}`,
        Icon,
        tone,
      })
    }
  }
  if (!cell.export) {
    statuses.push({
      id: 'not_exporting',
      labelKey: 'fontView.glyphStatus.notExporting',
      Icon: CircleOff,
      tone: 'neutral',
    })
  }
  return statuses
}
