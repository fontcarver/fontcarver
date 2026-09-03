import { readFileSync, readdirSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { describe, expect, it } from 'vitest'

const SRC = resolve(dirname(fileURLToPath(import.meta.url)), '..')

function sourceFiles(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name)
    if (entry.isDirectory()) return sourceFiles(path)
    return /\.tsx?$/.test(entry.name) ? [path] : []
  })
}

function classesOf(css: string): Set<string> {
  return new Set(
    [...css.matchAll(/\.([A-Za-z][\w-]*)/g)].map((match) => match[1] ?? ''),
  )
}

/**
 * Guards the per-component CSS modules: referencing a class that does not exist
 * type-checks and lints cleanly but silently renders unstyled, which is exactly
 * how splitting a shared stylesheet goes wrong.
 */
describe('CSS modules', () => {
  it('resolves every class referenced from a component', () => {
    const unresolved: string[] = []

    for (const file of sourceFiles(SRC)) {
      const source = readFileSync(file, 'utf8')
      const imports = [
        ...source.matchAll(/import\s+(\w+)\s+from\s+'([^']+\.module\.css)'/g),
      ]
      for (const [, alias = '', relative = ''] of imports) {
        const stylesheet = resolve(dirname(file), relative)
        const classes = classesOf(readFileSync(stylesheet, 'utf8'))
        for (const [, used = ''] of source.matchAll(
          new RegExp(`\\b${alias}\\.(\\w+)`, 'g'),
        ))
          if (!classes.has(used)) unresolved.push(`${file}: ${alias}.${used}`)
      }
    }

    expect(unresolved).toEqual([])
  })
})
