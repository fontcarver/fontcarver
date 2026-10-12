import { readdirSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

// macOS and Windows resolve paths case-insensitively, so `./PreviewSettings`
// can load `previewSettings.ts` there even though Linux finds the `.tsx`.
// Modules in one directory must therefore differ by more than letter case.
export function caseCollisions(files: string[]): string[][] {
  const groups = new Map<string, Set<string>>()
  for (const file of files) {
    const match = /^(.*?)(\.test)?\.(ts|tsx)$/.exec(file)
    if (!match) continue
    const stem = match[1] + (match[2] ?? '')
    const key = stem.toLowerCase()
    groups.set(key, (groups.get(key) ?? new Set()).add(stem))
  }
  return [...groups.values()]
    .filter((stems) => stems.size > 1)
    .map((stems) => [...stems].sort())
}

function collisionsUnder(directory: string): string[][] {
  const entries = readdirSync(directory, { withFileTypes: true })
  const files = entries
    .filter((entry) => entry.isFile())
    .map((entry) => entry.name)
  const nested = entries
    .filter((entry) => entry.isDirectory())
    .flatMap((entry) => collisionsUnder(join(directory, entry.name)))
  return [
    ...caseCollisions(files).map((group) =>
      group.map((stem) => join(directory, stem)),
    ),
    ...nested,
  ]
}

describe('module file names', () => {
  it('detects a component and a module whose names differ only in case', () => {
    expect(
      caseCollisions([
        'Sheet.tsx',
        'sheet.ts',
        'sheet.test.ts',
        'Sheet.test.tsx',
        'other.ts',
      ]),
    ).toEqual([
      ['Sheet', 'sheet'],
      ['Sheet.test', 'sheet.test'],
    ])
  })

  it('keeps every source directory free of case-only collisions', () => {
    expect(collisionsUnder(join(__dirname))).toEqual([])
  })
})
