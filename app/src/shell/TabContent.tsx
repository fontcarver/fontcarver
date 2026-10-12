import { BitmapView } from '../views/bitmap/BitmapView'
import type { DocSummary } from '../api'
import type { Tab } from '../stores/uiStore'
import { FeaturesView } from '../views/features/FeaturesView'
import { FontView } from '../views/font/FontView'
import { FontInfoPanel } from '../views/fontinfo/FontInfoPanel'
import { TextView } from '../views/text/TextView'

export interface TabContentProps {
  summary: DocSummary
  tab: Tab
}

/** Renders the view that belongs to the active tab. */
export function TabContent({ summary, tab }: TabContentProps) {
  if (tab.kind === 'bitmap')
    return <BitmapView key={tab.id} summary={summary} />
  if (tab.kind === 'glyph')
    return <TextView key={tab.id} summary={summary} tab={tab} />
  if (tab.kind === 'features')
    return <FeaturesView key={tab.id} summary={summary} />
  if (tab.kind === 'fontinfo') return <FontInfoPanel summary={summary} />
  return <FontView summary={summary} />
}
