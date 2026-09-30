import { expect, it } from 'vitest'

import {
  callSnippet,
  checkScript,
  findCallSites,
  scriptMethod,
  searchScriptMethods,
} from './scriptCheck'

it('exposes the generated contract of a real kernel method', () => {
  const method = scriptMethod('doc.apply')
  expect(method).toBeDefined()
  expect(method?.scriptable).toBe(true)
  expect(method?.closed).toBe(true)
  expect(method?.fields.map((field) => field.name).sort()).toEqual([
    'command',
    'doc',
    'label',
  ])
  expect(scriptMethod('doc.edit_begin')?.scriptable).toBe(false)
  expect(scriptMethod('automation.python_run')?.scriptable).toBe(false)
  expect(scriptMethod('doc.definitely_not_a_method')).toBeUndefined()
})

it('completes methods by name and by parameter key, excluding withheld ones', () => {
  const byName = searchScriptMethods('glyphs.')
  expect(byName.length).toBeGreaterThan(0)
  expect(byName.every((method) => method.method.startsWith('glyphs.'))).toBe(
    true,
  )
  expect(searchScriptMethods('automation.')).toEqual([])
  expect(searchScriptMethods('doc.edit_')).toEqual([])
  const exact = searchScriptMethods('doc.summary', 3)
  expect(exact[0]?.method).toBe('doc.summary')
})

it('builds a snippet that already contains every required parameter', () => {
  const method = scriptMethod('doc.apply')
  expect(method).toBeDefined()
  expect(callSnippet(method!)).toBe(
    "await fc.call('doc.apply', { doc: null, command: null, label: null })",
  )
  const empty = scriptMethod('session.list')
  expect(empty).toBeDefined()
  expect(callSnippet(empty!)).toBe("await fc.call('session.list', {})")
})

it('finds literal call sites and their parameter keys', () => {
  const sites = findCallSites(
    [
      'const a = await fc.call("session.list", {})',
      "await fontcarver . call( 'doc.summary' , { doc: 1 } )",
      'await fc.call(name, { doc: 1 })',
      'await fc.call("doc.apply", params)',
      'await fc.call("doc.apply", { ...params, doc: 1 })',
      'await fc.call("glyphs.list", { doc: 1, filter: { query: "a" } })',
    ].join('\n'),
  )
  expect(sites.map((site) => site.method)).toEqual([
    'session.list',
    'doc.summary',
    '',
    'doc.apply',
    'doc.apply',
    'glyphs.list',
  ])
  expect(sites[1]).toMatchObject({ line: 2, keys: ['doc'], spread: false })
  expect(sites[2]).toMatchObject({ line: 3, method: '' })
  expect(sites[3]).toMatchObject({ line: 4, keys: null })
  expect(sites[4]).toMatchObject({ line: 5, keys: ['doc'], spread: true })
  expect(sites[5].keys).toEqual(['doc', 'filter'])
})

it('reports unknown and withheld methods as blocking errors', () => {
  expect(checkScript('await fc.call("doc.nope", {})')).toEqual([
    {
      severity: 'error',
      line: 1,
      key: 'automation.check.unknownMethod',
      values: { method: 'doc.nope' },
    },
  ])
  expect(
    checkScript('await fc.call("doc.edit_begin", { doc: 1, label: "x" })'),
  ).toEqual([
    {
      severity: 'error',
      line: 1,
      key: 'automation.check.unavailableMethod',
      values: { method: 'doc.edit_begin' },
    },
  ])
})

it('reports an unknown parameter key as a non-blocking warning', () => {
  expect(
    checkScript('await fc.call("doc.apply", { doc: 1, oops: 2 })'),
  ).toEqual([
    {
      severity: 'warning',
      line: 1,
      key: 'automation.check.unknownParameter',
      values: { method: 'doc.apply', parameter: 'oops' },
    },
  ])
})

it('never reports a key that a serde default makes optional as missing', () => {
  // `glyphs.list` declares `filter` and `sort` with `#[serde(default)]`; the
  // generated TypeScript cannot say so, so omitting them must not be flagged.
  expect(
    checkScript(
      'await fc.call("glyphs.list", { doc: 1, offset: 0, limit: 10 })',
    ),
  ).toEqual([])
  expect(checkScript('await fc.call("doc.apply", { doc: 1 })')).toEqual([])
})

it('accepts a complete call and never guesses past a spread or a variable', () => {
  expect(
    checkScript(
      'await fc.call("doc.apply", { doc: 1, command: {}, label: "x" })',
    ),
  ).toEqual([])
  expect(checkScript('await fc.call("doc.apply", { ...request })')).toEqual([])
  expect(checkScript('await fc.call("doc.apply", request)')).toEqual([])
  expect(checkScript('await fc.call(method, request)')).toEqual([])
})

it('never reports a call written inside a comment or a string', () => {
  expect(
    checkScript(
      [
        '// await fc.call("doc.nope", {})',
        '/* fc.call("doc.nope", {})',
        '   still a comment */',
        'const text = \'fc.call("doc.nope", {})\'',
        'await fc.call("session.list", {})',
      ].join('\n'),
    ),
  ).toEqual([])
})

it('keeps reported line numbers correct after multi-line calls', () => {
  expect(
    checkScript(
      [
        'await fc.call(',
        '  "doc.apply",',
        '  request,',
        ')',
        'fc.call("doc.nope", {})',
      ].join('\n'),
    ),
  ).toEqual([
    {
      severity: 'error',
      line: 5,
      key: 'automation.check.unknownMethod',
      values: { method: 'doc.nope' },
    },
  ])
})

it('accepts shorthand properties and ignores identifiers used as values', () => {
  expect(
    checkScript(
      'const doc = 1\nconst command = {}\nawait fc.call("doc.apply", { doc, command, label: "x" })',
    ),
  ).toEqual([])
  // A bare identifier in value position must not be read as a property name.
  expect(
    checkScript('await fc.call("doc.apply", { doc: docId, command, label })'),
  ).toEqual([])
  expect(
    findCallSites('fc.call("doc.apply", { doc: docId, command, label })')[0]
      .keys,
  ).toEqual(['doc', 'command', 'label'])
})

it('marks a nullable generated field optional and still rejects unknown keys', () => {
  const outline = scriptMethod('glyphs.outline')
  expect(
    outline?.fields.find((field) => field.name === 'layer')?.optional,
  ).toBe(true)
  expect(checkScript('await fc.call("glyphs.outline", { doc, gid })')).toEqual(
    [],
  )
  expect(
    checkScript('await fc.call("glyphs.outline", { doc, gid, master: null })'),
  ).toEqual([
    {
      severity: 'warning',
      line: 1,
      key: 'automation.check.unknownParameter',
      values: { method: 'glyphs.outline', parameter: 'master' },
    },
  ])
})
