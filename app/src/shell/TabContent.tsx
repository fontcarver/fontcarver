import type { DocSummary } from '../api'
import type { Tab } from '../stores/uiStore'
import { FontView } from '../views/font/FontView'
import { FontInfoPlaceholder } from '../views/fontinfo/FontInfoPlaceholder'
import { GlyphViewPlaceholder } from '../views/glyph/GlyphViewPlaceholder'

export interface TabContentProps {
  summary: DocSummary
  tab: Tab
}

/** Renders the view that belongs to the active tab. */
export function TabContent({ summary, tab }: TabContentProps) {
  if (tab.kind === 'glyph')
    return <GlyphViewPlaceholder summary={summary} gid={tab.gid} />
  if (tab.kind === 'fontinfo') return <FontInfoPlaceholder summary={summary} />
  return <FontView summary={summary} />
}
