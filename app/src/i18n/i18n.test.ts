import { describe, expect, it } from 'vitest'

import en from './en.json'
import zhCN from './zh-CN.json'

type Json = { [key: string]: string | Json }

function keyPaths(value: Json, prefix = ''): string[] {
  return Object.entries(value).flatMap(([key, child]) => {
    const path = prefix ? `${prefix}.${key}` : key
    return typeof child === 'string' ? [path] : keyPaths(child, path)
  })
}

describe('translation catalogues', () => {
  it('gives every English key a zh-CN counterpart', () => {
    const english = keyPaths(en as Json).sort()
    const chinese = keyPaths(zhCN as Json).sort()

    expect(chinese).toEqual(english)
  })

  it('keeps interpolation placeholders identical across catalogues', () => {
    const flatten = (value: Json, prefix = ''): [string, string][] =>
      Object.entries(value).flatMap(([key, child]) => {
        const path = prefix ? `${prefix}.${key}` : key
        return typeof child === 'string'
          ? ([[path, child]] as [string, string][])
          : flatten(child, path)
      })
    const placeholders = (text: string) =>
      [...text.matchAll(/\{\{\s*(\w+)[^}]*\}\}/g)]
        .map((match) => match[1])
        .sort()

    const chinese = new Map(flatten(zhCN as Json))
    for (const [key, value] of flatten(en as Json))
      expect(placeholders(chinese.get(key) ?? '')).toEqual(placeholders(value))
  })
})
