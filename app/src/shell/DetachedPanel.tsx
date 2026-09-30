import type { DocSummary } from '../api'
import type { Tab } from '../stores/uiStore'
import type { WindowTarget } from '../transport/windows'
import { PreviewWindow } from '../views/text/PreviewWindow'
import { DetachedGlyphSource } from './DetachedGlyphSource'
import { Inspector } from './Inspector'
import { Sidebar } from './Sidebar'

/**
 * Renders the selected contextual panel against its shared document.
 *
 * A sidebar or Inspector torn off an Edit tab has no canvas beside it, and the
 * editing panels read the glyph the canvas publishes. `DetachedGlyphSource`
 * publishes it here instead, so the panel shows the same live content as the
 * dock it came from and follows the glyph the tab moves to.
 */
export function DetachedPanel({
  target,
  summary,
  tab,
}: {
  target: Extract<WindowTarget, { kind: 'panel' }>
  summary: DocSummary
  tab: Tab
}) {
  if (target.panel === 'preview')
    return <PreviewWindow summary={summary} tab={tab} />

  // Only an Edit tab has a glyph to publish. This window has no canvas of its
  // own, so the source stays mounted for as long as the panel does rather than
  // unmounting itself once it has published.
  const source =
    tab.kind === 'glyph' && tab.gid !== undefined ? (
      <DetachedGlyphSource summary={summary} gid={tab.gid} />
    ) : null

  return (
    <>
      {source}
      {target.panel === 'sidebar' ? <Sidebar /> : <Inspector />}
    </>
  )
}
