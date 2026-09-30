import type { DocSummary } from '../api'
import type { Tab } from '../stores/uiStore'
import type { WindowTarget } from '../transport/windows'
import { PreviewWindow } from '../views/text/PreviewWindow'
import { Inspector } from './Inspector'
import { Sidebar } from './Sidebar'

/** Renders the selected contextual panel against its shared document. */
export function DetachedPanel({
  target,
  summary,
  tab,
}: {
  target: Extract<WindowTarget, { kind: 'panel' }>
  summary: DocSummary
  tab: Tab
}) {
  if (target.panel === 'sidebar') return <Sidebar />
  if (target.panel === 'inspector') return <Inspector />
  return <PreviewWindow summary={summary} tab={tab} />
}
