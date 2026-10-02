import { expect } from '@playwright/test'

import type { DocumentHistory } from '../src/api/generated/DocumentHistory'
import type { LayerOutline } from '../src/api/generated/LayerOutline'
import type { Point } from '../src/api/generated/Point'
import { sourceTest as test } from './sessionFixtures'
import { api } from './support'

const point = (x: number, y: number, kind: Point['kind'] = 'Line'): Point => ({
  x,
  y,
  kind,
  smooth: false,
  name: null,
  tt_index: 'Unnumbered',
})

test('Fix All resolves combined defects with one undo and preserves path locks', async ({
  page,
}, testInfo) => {
  test.setTimeout(90_000)
  await page.goto('/')
  const family = 'Audit repair batch'
  const { doc } = await api<{ doc: number }>(page, 'session.new', {
    family,
    units_per_em: 1000,
  })
  const added = await api<{ added: { gid: number }[] }>(page, 'glyphs.add', {
    doc,
    recipe: 'A',
    master: null,
    dry_run: false,
  })
  const gid = added.added[0]!.gid
  const read = () =>
    api<LayerOutline>(page, 'glyphs.outline', { doc, gid, layer: null })
  const { layer } = await read()
  await api(page, 'doc.apply', {
    doc,
    label: 'Defects and locked outline',
    command: {
      Batch: [
        {
          AddContour: {
            glyph: 'A',
            layer,
            contour: {
              closed: false,
              name: null,
              id: 'repair',
              points: [
                point(0.2, 0, 'Move'),
                point(300.2, 400, 'OffCurve'),
                point(-299.8, 400, 'OffCurve'),
                point(0.2, 800, 'Curve'),
                point(0.2, 800),
              ],
            },
          },
        },
        {
          AddContour: {
            glyph: 'A',
            layer,
            contour: {
              closed: true,
              name: null,
              id: 'locked',
              points: [point(1000.2, 0), point(1100, 0), point(1100, 100)],
            },
          },
        },
        {
          EditorMetadata: {
            PathLocks: { glyph: 'A', layer, contours: [1], locked: true },
          },
        },
      ],
    },
  })
  const before = await read()
  expect(before.contours[0]!.closed).toBe(false)
  expect(before.contours[0]!.points).toHaveLength(5)
  const historyBefore = await api<DocumentHistory>(page, 'doc.history', { doc })
  await page.reload()
  const tab = page.getByRole('tab', { name: family })
  await expect(tab).toBeVisible()
  await tab.click()
  await page
    .getByRole('listbox', { name: 'Glyphs' })
    .getByRole('option', { name: 'A', exact: true })
    .dblclick()
  await expect(page.getByRole('img', { name: /^Glyph A/ })).toBeVisible()
  await page.getByText(/Outline audit ·/).click()
  await page.getByRole('button', { name: 'Fix All', exact: true }).click()
  await expect
    .poll(async () => {
      const contour = (await read()).contours[0]!
      return (
        contour.closed &&
        contour.points.length === 10 &&
        contour.points.every(
          (p) => Number.isInteger(p.x) && Number.isInteger(p.y),
        )
      )
    })
    .toBe(true)
  await expect(
    page.getByRole('button', { name: 'Fix All', exact: true }),
  ).toBeDisabled()
  const after = await read()
  expect(after.contours[0]!.closed).toBe(true)
  expect(
    after.contours[0]!.points.every(
      (p) => Number.isInteger(p.x) && Number.isInteger(p.y),
    ),
  ).toBe(true)
  const extremes = after.contours[0]!.points.filter(
    (p) => p.kind !== 'OffCurve' && Math.abs(p.x) > 80,
  )
  expect(extremes).toHaveLength(2)
  expect(after.contours[1]).toEqual(before.contours[1])
  const historyAfter = await api<DocumentHistory>(page, 'doc.history', { doc })
  expect(historyAfter.len).toBe(historyBefore.len + 1)
  expect(historyAfter.cursor).toBe(historyBefore.cursor + 1)
  await api(page, 'doc.undo', { doc })
  expect((await read()).contours).toEqual(before.contours)
  await api(page, 'doc.redo', { doc })
  expect((await read()).contours).toEqual(after.contours)
  const path = testInfo.outputPath('repaired.fcf')
  await api(page, 'session.save', { doc, path })
  const reopened = await api<{ doc: number }>(page, 'session.open', {
    path,
    face_index: null,
  })
  const saved = await api<LayerOutline>(page, 'glyphs.outline', {
    doc: reopened.doc,
    gid,
    layer,
  })
  expect(saved.contours).toEqual(after.contours)
  await api(page, 'session.close', { doc: reopened.doc, force: true })
})
