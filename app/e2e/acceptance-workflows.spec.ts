import { existsSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { basename, join } from 'node:path'

import { expect, test } from '@playwright/test'

import { api, fixture } from './support'
import {
  closePenContourWithPointer,
  docs,
  exportThroughDialog,
  firstOnCurve,
  glyphCell,
  menuItem,
  movePointWithPointer,
  openGlyphFromGrid,
  openPathThroughDialog,
  outline,
  releaseGlyphs,
  releaseInspect,
  type PointLocation,
} from './acceptanceSupport'

const DEJAVU = fixture('DejaVuSans.ttf')

test.describe.serial('P1 browser acceptance workflows', () => {
  test('opens DejaVu, pointer-edits A/B/C, exports and reopens', async ({
    page,
  }) => {
    test.setTimeout(180_000)
    const directory = mkdtempSync(join(tmpdir(), 'fc-e2e-real-'))
    const output = join(directory, 'EditedDejaVuSans.ttf')
    const ownedDocs: number[] = []
    await page.goto('/')
    try {
      const source = await openPathThroughDialog(page, DEJAVU)
      ownedDocs.push(source.doc)
      const cells = new Map(
        await Promise.all(
          ['A', 'B', 'C'].map(
            async (name) =>
              [name, await glyphCell(page, source.doc, name)] as const,
          ),
        ),
      )
      const original = new Map<string, PointLocation>()
      const moved = new Map<string, PointLocation>()

      await openGlyphFromGrid(page, 'A')
      for (const [index, name] of ['A', 'B', 'C'].entries()) {
        const cell = cells.get(name)
        if (!cell) throw new Error(`missing ${name}`)
        if (index > 0) {
          await page.keyboard.press('Control+]')
          await expect(
            page.getByRole('img', { name: new RegExp(`^Glyph ${name}`) }),
          ).toBeVisible()
        }
        const before = firstOnCurve(await outline(page, source.doc, cell.gid))
        original.set(name, before)
        moved.set(
          name,
          await movePointWithPointer(page, source.doc, cell.gid, before, {
            x: 12 + index * 2,
            y: 6 + index,
          }),
        )
      }

      await exportThroughDialog(page, output)
      expect(existsSync(output), 'dialog wrote the edited TTF').toBe(true)
      const inspected = releaseInspect(output)
      expect(inspected.summary.glyph_count).toBe(source.glyph_count)
      expect(inspected.summary.family_name).toBe('DejaVu Sans')

      const reopened = await openPathThroughDialog(page, output)
      ownedDocs.push(reopened.doc)
      for (const name of ['A', 'B', 'C']) {
        const cell = await glyphCell(page, reopened.doc, name)
        const actual = (
          await outline(page, reopened.doc, cell.gid)
        ).contours.flatMap((contour) => contour.points)
        const after = moved.get(name) as PointLocation
        const before = original.get(name) as PointLocation
        expect(after.x).not.toBe(before.x)
        expect(after.y).not.toBe(before.y)
        expect(
          actual.some(
            (point) =>
              Math.abs(point.x - after.x) < 0.51 &&
              Math.abs(point.y - after.y) < 0.51,
          ),
          `${name} reopened with its pointer-moved coordinate`,
        ).toBe(true)
      }
    } finally {
      for (const doc of ownedDocs) {
        await api(page, 'session.close', { doc, force: true }).catch(() => {})
      }
      rmSync(directory, { recursive: true, force: true })
    }
  })

  test('creates A-C, pen-draws and widths them, exports and reopens', async ({
    page,
  }) => {
    test.setTimeout(180_000)
    const directory = mkdtempSync(join(tmpdir(), 'fc-e2e-scratch-'))
    const output = join(directory, 'BrowserScratch.ttf')
    const family = `Browser Scratch ${Date.now()}`
    const ownedDocs: number[] = []
    await page.goto('/')
    try {
      await menuItem(page, 'File', /^New Font/)
      const create = page.getByRole('dialog', { name: 'New Font' })
      await create.getByLabel('Family name').fill(family)
      await create.getByRole('radio', { name: 'None', exact: true }).check()
      await create.getByRole('button', { name: 'Create' }).click()
      await expect(page.getByRole('tab', { name: family })).toBeVisible()
      const created = (await docs(page)).find(
        (doc) => doc.family_name === family,
      )
      if (!created) throw new Error('new document missing from session')
      ownedDocs.push(created.doc)

      await menuItem(page, 'Glyph', /^Add Glyphs/)
      const add = page.getByRole('dialog', { name: 'Add Glyphs' })
      await add.getByLabel('Recipe').fill('A-C')
      const addButton = add.getByRole('button', { name: 'Add 3 glyphs' })
      await expect(addButton).toBeEnabled()
      await addButton.click()
      await expect(add).toBeHidden()

      for (const [index, name] of ['A', 'B', 'C'].entries()) {
        await page.getByRole('tab', { name: family }).click()
        await openGlyphFromGrid(page, name)
        const cell = await glyphCell(page, created.doc, name)
        const canvas = page.getByRole('img', {
          name: new RegExp(`^Glyph ${name}`),
        })
        await page.getByRole('button', { name: 'Pen', exact: true }).click()
        await expect(
          page.getByRole('button', { name: 'Pen', exact: true }),
        ).toHaveAttribute('aria-pressed', 'true')
        const bounds = await canvas.boundingBox()
        if (!bounds) throw new Error('canvas has no bounds')
        // Draw on exposed canvas above the bottom-left Info box.
        const corners = [
          { x: bounds.width * 0.45, y: bounds.height * 0.25 },
          { x: bounds.width * 0.75, y: bounds.height * 0.25 },
          { x: bounds.width * 0.6, y: bounds.height * 0.45 },
        ]
        for (const corner of corners) await canvas.click({ position: corner })
        await closePenContourWithPointer(page, canvas)
        await expect
          .poll(
            async () =>
              (await outline(page, created.doc, cell.gid)).contours.length,
          )
          .toBe(1)

        const width = 600 + index * 20
        const advance = page
          .getByRole('region', { name: 'Info box' })
          .getByLabel('advance')
        await advance.fill(String(width))
        await advance.press('Enter')
        await expect
          .poll(async () => (await outline(page, created.doc, cell.gid)).width)
          .toBe(width)
      }

      await exportThroughDialog(page, output)
      expect(existsSync(output), 'dialog wrote the scratch TTF').toBe(true)
      const inspected = releaseInspect(output)
      expect(inspected.summary.family_name).toBe(family)
      expect(inspected.summary.glyph_count).toBe(4)
      const listed = releaseGlyphs(output)
      for (const [index, name] of ['A', 'B', 'C'].entries())
        expect(listed).toContain(
          `${name}\tU+004${index + 1}\t${600 + index * 20}`,
        )

      const reopened = await openPathThroughDialog(page, output)
      ownedDocs.push(reopened.doc)
      expect(basename(reopened.path ?? '')).toBe('BrowserScratch.ttf')
      for (const [index, name] of ['A', 'B', 'C'].entries()) {
        const cell = await glyphCell(page, reopened.doc, name)
        const actual = await outline(page, reopened.doc, cell.gid)
        expect(actual.width).toBe(600 + index * 20)
        expect(actual.contours).toHaveLength(1)
        expect(actual.contours[0]?.closed).toBe(true)
        expect(actual.contours[0]?.points).toHaveLength(3)
      }
    } finally {
      for (const doc of ownedDocs) {
        await api(page, 'session.close', { doc, force: true }).catch(() => {})
      }
      rmSync(directory, { recursive: true, force: true })
    }
  })
})
