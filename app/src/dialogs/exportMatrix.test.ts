import { describe, expect, it, vi } from 'vitest'

import type { ExportOptions, ExportReport } from '../api'

import {
  planExportMatrix,
  runExportMatrix,
  type ExportMatrixCell,
} from './exportMatrix'

function options(overrides: Partial<ExportOptions> = {}): ExportOptions {
  return {
    format: 'Woff2',
    variable: false,
    master: null,
    remove_overlap: true,
    keep_tt_instructions: false,
    preserve_untouched_tables: true,
    production_names: true,
    autohint: false,
    table_subset: null,
    ...overrides,
  }
}

/** Cells as `export.resolve_profile` returns them. */
function cells(): ExportMatrixCell[] {
  return [
    {
      profileName: 'Web',
      rowKind: 'instance',
      rowName: 'Regular',
      options: options({ instance: 0 }),
      relativePath: 'web/MyFont-Regular.woff2',
    },
    {
      profileName: 'Web',
      rowKind: 'instance',
      rowName: 'Bold',
      options: options({ instance: 2 }),
      relativePath: 'web/MyFont-Bold.woff2',
    },
    {
      profileName: 'Web',
      rowKind: 'variable',
      rowName: 'Text VF',
      options: options({ variable: true, axis_subset: ['wght'] }),
      relativePath: 'web/MyFont-Text.woff2',
    },
  ]
}

function report(path: string): ExportReport {
  return {
    path,
    format: 'Woff2',
    bytes_written: 12,
    warnings: [],
    tables: [],
    touched_glyphs: 0,
    timing_ms: 1,
  }
}

describe('planExportMatrix', () => {
  it('joins the destination with every kernel-resolved relative path', () => {
    const source = cells()
    const original = structuredClone(source)
    const items = planExportMatrix('/exports/', source)
    expect(items.map((item) => item.path)).toEqual([
      '/exports/web/MyFont-Regular.woff2',
      '/exports/web/MyFont-Bold.woff2',
      '/exports/web/MyFont-Text.woff2',
    ])
    expect(new Set(items.map((item) => item.id)).size).toBe(3)
    expect(items[2]).toMatchObject({
      profileName: 'Web',
      rowKind: 'variable',
      rowName: 'Text VF',
      options: { variable: true, axis_subset: ['wght'] },
    })
    // The plan is a snapshot: mutating it must not reach the caller's cells.
    items[0]!.options.production_names = false
    expect(source).toEqual(original)
  })

  it('uses the separator the destination itself uses', () => {
    const items = planExportMatrix('C:\\fonts\\out', cells())
    expect(items[0]?.path).toBe('C:\\fonts\\out\\web\\MyFont-Regular.woff2')
  })

  it('requires a destination and at least one cell', () => {
    expect(() => planExportMatrix('   ', cells())).toThrowError(
      /output directory/i,
    )
    expect(() => planExportMatrix('/exports', [])).toThrowError(
      /at least one instance/i,
    )
  })

  it('refuses a resolved path that would leave the destination', () => {
    for (const relativePath of [
      '../escape.woff2',
      '/absolute.woff2',
      'C:/drive.woff2',
      'a//b.woff2',
      './here.woff2',
      '   ',
    ]) {
      const [first] = cells()
      expect(() =>
        planExportMatrix('/exports', [{ ...first!, relativePath }]),
      ).toThrowError()
    }
  })

  it('refuses two cells that resolve to the same destination', () => {
    const [first, second] = cells()
    expect(() =>
      planExportMatrix('/exports', [
        first!,
        { ...second!, relativePath: first!.relativePath.toUpperCase() },
      ]),
    ).toThrowError(/collision/i)
  })

  it('refuses a plan larger than the batch limit', () => {
    const many = Array.from({ length: 101 }, (_, index) => ({
      ...cells()[0]!,
      rowName: `Instance ${index}`,
      relativePath: `web/font-${index}.woff2`,
    }))
    expect(() => planExportMatrix('/exports', many)).toThrowError(/100/)
  })
})

describe('runExportMatrix', () => {
  it('exports every item in order and keeps individual failures', async () => {
    const items = planExportMatrix('/exports', cells())
    const seen: string[] = []
    const results = await runExportMatrix(items, (item) => {
      seen.push(item.path)
      if (item.rowName === 'Bold') return Promise.reject(new Error('boom'))
      return Promise.resolve(report(item.path))
    })
    expect(seen).toEqual(items.map((item) => item.path))
    expect(results.map((result) => result.report !== null)).toEqual([
      true,
      false,
      true,
    ])
    expect(results[1]?.error).toBeInstanceOf(Error)
  })

  it('stops scheduling new items once the caller says to stop', async () => {
    const items = planExportMatrix('/exports', cells())
    const run = vi.fn((item: { path: string }) =>
      Promise.resolve(report(item.path)),
    )
    let allowed = 1
    const results = await runExportMatrix(
      items,
      run as unknown as (item: { path: string }) => Promise<ExportReport>,
      undefined,
      () => allowed-- > 0,
    )
    expect(results).toHaveLength(1)
    expect(run).toHaveBeenCalledOnce()
  })

  it('reports each finished item as it finishes', async () => {
    const items = planExportMatrix('/exports', cells())
    const finished: string[] = []
    await runExportMatrix(
      items,
      (item) => Promise.resolve(report(item.path)),
      (result) => finished.push(result.item.rowName),
    )
    expect(finished).toEqual(['Regular', 'Bold', 'Text VF'])
  })
})
