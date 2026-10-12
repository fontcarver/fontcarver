import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { expect, it } from 'vitest'

import { CONSTRUCTION_PREFIXES } from './ConstructionSources'

/**
 * The convention has one source of truth, `fc_model::CONSTRUCTION_PREFIXES`.
 *
 * The editor needs the prefix for a kind the user has chosen but not yet
 * created, so there is no API answer to read it from; this test reads the
 * kernel's own table instead, in the kind order `ConstructionKind` declares, so
 * the two cannot drift apart without a failure.
 */
function kernelPrefixes(): string[] {
  // Vitest runs with the `app` directory as its root.
  const source = readFileSync(
    resolve(process.cwd(), '../crates/fc-model/src/construction.rs'),
    'utf8',
  )
  const table = source.match(
    /pub const CONSTRUCTION_PREFIXES: \[&str; 4\] = \[([^\]]*)\]/,
  )
  expect(table, 'the kernel still declares CONSTRUCTION_PREFIXES').toBeTruthy()
  return [...(table?.[1] ?? '').matchAll(/"([^"]+)"/g)].map(
    (match) => match[1] as string,
  )
}

it('uses exactly the prefixes the kernel reserves, in kind order', () => {
  expect(Object.values(CONSTRUCTION_PREFIXES)).toEqual(kernelPrefixes())
  expect(Object.keys(CONSTRUCTION_PREFIXES)).toEqual([
    'corner',
    'cap',
    'segment',
    'brush',
  ])
})

it('fails when a prefix drifts from the kernel table', () => {
  const drifted = { ...CONSTRUCTION_PREFIXES, corner: '_serif.' }
  expect(Object.values(drifted)).not.toEqual(kernelPrefixes())
})
