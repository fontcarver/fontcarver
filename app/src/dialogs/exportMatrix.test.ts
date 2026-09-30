import { describe, expect, it, vi } from 'vitest'

import type { ExportOptions, ExportReport } from '../api'
import type { ExportProfile } from '../stores/exportProfileStore'

import { planExportMatrix, runExportMatrix } from './exportMatrix'

function profile(overrides: Partial<ExportProfile> = {}): ExportProfile {
  return {
    id: 'web',
    name: 'Web',
    options: {
      format: 'Woff2',
      variable: true,
      master: 'master-one',
      master_subset: ['master-one'],
      remove_overlap: true,
      keep_tt_instructions: true,
      preserve_untouched_tables: true,
      production_names: true,
      autohint: false,
      table_subset: ['name'],
      bitmap_strike: [16, 1],
    },
    ...overrides,
  }
}

const instances = [
  { index: 0, name: 'Regular' },
  { index: 2, name: 'Bold' },
]

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
  it('plans independent static options for each selected named instance', () => {
    const source = profile()
    const original = structuredClone(source)
    const items = planExportMatrix('/exports/', [source], instances)
    expect(items.map((item) => item.path)).toEqual([
      '/exports/Regular-Web.woff2',
      '/exports/Bold-Web.woff2',
    ])
    expect(new Set(items.map((item) => item.id)).size).toBe(2)
    expect(items[1]).toMatchObject({
      profileName: 'Web',
      instanceName: 'Bold',
      options: {
        instance: 2,
        variable: false,
        master: null,
        master_subset: undefined,
        keep_tt_instructions: false,
      },
    })
    items[0].options.table_subset?.push('cmap')
    items[0].options.bitmap_strike![0] = 32
    expect(items[1].options.table_subset).toEqual(['name'])
    expect(items[1].options.bitmap_strike).toEqual([16, 1])
    expect(source).toEqual(original)
  })

  it('sanitizes names and handles Windows and filesystem-root directories', () => {
    expect(
      planExportMatrix(
        'C:\\exports\\',
        [profile({ name: 'Web/Small' })],
        [{ index: 0, name: '../Bold\\Heavy' }],
      )[0].path,
    ).toBe('C:\\exports\\Bold-Heavy-Web-Small.woff2')
    expect(planExportMatrix('/', [profile()], instances)[0].path).toBe(
      '/Regular-Web.woff2',
    )
  })

  it.each([
    ['Regular', 'regular'],
    ['Semi/Bold', 'Semi\\Bold'],
    ['Café', 'Cafe\u0301'],
  ])('rejects colliding instance names %s and %s', (left, right) => {
    expect(() =>
      planExportMatrix(
        '/exports',
        [profile()],
        [
          { index: 0, name: left },
          { index: 1, name: right },
        ],
      ),
    ).toThrow(/collision/i)
  })

  it('rejects profile collisions after sanitizing filenames', () => {
    expect(() =>
      planExportMatrix(
        '/exports',
        [profile({ name: 'Web:Small' }), profile({ name: 'Web?Small' })],
        instances,
      ),
    ).toThrow(/collision/i)
  })

  it('rejects ambiguous or empty selections and unsupported formats', () => {
    expect(() => planExportMatrix('  ', [profile()], instances)).toThrow(
      /directory/i,
    )
    expect(() => planExportMatrix('/exports', [], instances)).toThrow(
      /profile/i,
    )
    expect(() => planExportMatrix('/exports', [profile()], [])).toThrow(
      /instance/i,
    )
    for (const index of [-1, 1.2, NaN, Infinity, Number.MAX_SAFE_INTEGER + 1]) {
      expect(() =>
        planExportMatrix('/exports', [profile()], [{ index, name: 'Regular' }]),
      ).toThrow(/index/i)
    }
    expect(() =>
      planExportMatrix('/exports', [profile()], [instances[0], instances[0]]),
    ).toThrow(/index/i)
    expect(() =>
      planExportMatrix(
        '/exports',
        [profile({ options: { ...profile().options, format: 'Ufo' } })],
        instances,
      ),
    ).toThrow(/format/i)
  })

  it('accepts 100 outputs and rejects a larger matrix', () => {
    const many = Array.from({ length: 100 }, (_, index) => ({
      index,
      name: `Instance ${index}`,
    }))
    expect(planExportMatrix('/exports', [profile()], many)).toHaveLength(100)
    expect(() =>
      planExportMatrix(
        '/exports',
        [profile()],
        [...many, { index: 100, name: 'Overflow' }],
      ),
    ).toThrow(/100/)
  })
})

describe('runExportMatrix', () => {
  it('continues after failures and reports each result in order', async () => {
    const items = planExportMatrix('/exports', [profile()], instances)
    const failure = new Error('Disk full')
    const exportItem = vi
      .fn()
      .mockRejectedValueOnce(failure)
      .mockImplementationOnce(async (item) => report(item.path))
    const onResult = vi.fn()
    const results = await runExportMatrix(items, exportItem, onResult)
    expect(exportItem).toHaveBeenCalledTimes(2)
    expect(results[0]).toEqual({ item: items[0], report: null, error: failure })
    expect(results[1]).toEqual({
      item: items[1],
      report: report(items[1].path),
      error: null,
    })
    expect(onResult.mock.calls.map(([result]) => result)).toEqual(results)
  })

  it('snapshots all work before the first awaited export', async () => {
    const items = planExportMatrix('/exports', [profile()], instances)
    const paths: string[] = []
    const options: ExportOptions[] = []
    let finishFirst!: () => void
    const first = new Promise<void>((resolve) => {
      finishFirst = resolve
    })
    const running = runExportMatrix(items, async (item) => {
      paths.push(item.path)
      options.push(item.options)
      if (paths.length === 1) await first
      return report(item.path)
    })
    items[1].path = '/elsewhere/changed.woff2'
    items[1].options.table_subset?.push('cmap')
    items.pop()
    expect(paths).toHaveLength(1)
    finishFirst()
    await running
    expect(paths).toEqual([
      '/exports/Regular-Web.woff2',
      '/exports/Bold-Web.woff2',
    ])
    expect(options[1].table_subset).toEqual(['name'])
  })

  it('validates the entire batch before performing any export', async () => {
    const items = planExportMatrix('/exports', [profile()], instances)
    items[1].path = '\\EXPORTS\\Regular-Web.woff2'
    const exportItem = vi.fn()
    await expect(runExportMatrix(items, exportItem)).rejects.toThrow(
      /collision/i,
    )
    expect(exportItem).not.toHaveBeenCalled()
  })

  it('stops before another export when continuation is revoked', async () => {
    const items = planExportMatrix('/exports', [profile()], instances)
    let active = true
    const exportItem = vi.fn(async (item) => {
      active = false
      return report(item.path)
    })
    const results = await runExportMatrix(
      items,
      exportItem,
      undefined,
      () => active,
    )
    expect(exportItem).toHaveBeenCalledTimes(1)
    expect(results).toHaveLength(1)
  })
})
