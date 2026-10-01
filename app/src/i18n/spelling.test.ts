import { describe, expect, it } from 'vitest'

import en from './en.json'

/**
 * The English catalogue is written in one spelling.
 *
 * It was mixed: "Licence" beside six "License", "Colour" beside thirteen
 * "Color", "Centre" beside "Center", "behaviour" beside "behavior". American
 * spelling was already the majority in every pair, so that is the one the
 * catalogue keeps, and this fails when a new string reintroduces the other.
 * The reader sees these strings side by side in the same menu, which is how the
 * drift was noticed in the first place.
 */
const BRITISH: readonly [RegExp, string][] = [
  [/\bcolour/i, 'color'],
  [/\blicence/i, 'license'],
  [/\bcentre/i, 'center'],
  [/\bbehaviour/i, 'behavior'],
  [/\bgrey\b/i, 'gray'],
  [/\bcatalogue/i, 'catalog'],
  [/\bdefence/i, 'defense'],
  [/\bfavourite/i, 'favorite'],
  [/\bnormalis/i, 'normaliz'],
  [/\borganis/i, 'organiz'],
  [/\bcustomis/i, 'customiz'],
  [/\brecognis/i, 'recogniz'],
  [/\boptimis/i, 'optimiz'],
  [/\bsynchronis/i, 'synchroniz'],
  [/\banalyse/i, 'analyze'],
]

function leaves(value: unknown, path = ''): [string, string][] {
  if (typeof value === 'string') return [[path, value]]
  if (typeof value !== 'object' || value === null) return []
  return Object.entries(value as Record<string, unknown>).flatMap(
    ([key, item]) => leaves(item, path ? `${path}.${key}` : key),
  )
}

describe('English catalogue spelling', () => {
  it('uses one spelling throughout', () => {
    const offenders: string[] = []
    for (const [key, text] of leaves(en))
      for (const [pattern, preferred] of BRITISH) {
        const match = pattern.exec(text)
        if (!match) continue
        offenders.push(`${key}: ${match[0]} → ${preferred}`)
      }
    expect(offenders).toEqual([])
  })
})
