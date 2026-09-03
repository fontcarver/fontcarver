import { CountRow } from '../components/CountRow'
import { Panel } from '../components/Panel'

export interface CountPanelProps {
  title: string
  entries: readonly [string, number][]
}

/** A sidebar section listing labelled glyph counts; renders nothing when empty. */
export function CountPanel({ title, entries }: CountPanelProps) {
  if (entries.length === 0) return null
  return (
    <Panel title={title}>
      {entries.map(([label, value]) => (
        <CountRow key={label} label={label} value={value} />
      ))}
    </Panel>
  )
}
