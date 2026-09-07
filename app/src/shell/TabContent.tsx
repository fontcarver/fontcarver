import type { DocSummary } from '../api'
import type { Tab } from '../stores/uiStore'
import { FontView } from '../views/font/FontView'
import { FontInfoPanel } from '../views/fontinfo/FontInfoPanel'
import { TextView } from '../views/text/TextView'

export interface TabContentProps {
  summary: DocSummary
  tab: Tab
}

/** Renders the view that belongs to the active tab. */
export function TabContent({ summary, tab }: TabContentProps) {
  if (tab.kind === 'glyph')
    return <TextView key={tab.id} summary={summary} tab={tab} />
  if (tab.kind === 'fontinfo') return <FontInfoPanel summary={summary} />
  return <FontView summary={summary} />
}
