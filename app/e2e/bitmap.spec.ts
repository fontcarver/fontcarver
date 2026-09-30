import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test, expect, type Page } from '@playwright/test'
import { api } from './support'
import { menuItem, openPathThroughDialog } from './acceptanceSupport'
import type { BitmapGlyphView, BitmapState } from '../src/api/generated'
const sample = `STARTFONT 2.1
FONT -FontCarver-BitmapTest-Medium-R-Normal--8-80-72-72-C-80-ISO10646-1
SIZE 8 72 72
FONTBOUNDINGBOX 8 8 0 -2
STARTPROPERTIES 4
FONT_ASCENT 6
FONT_DESCENT 2
CHARSET_REGISTRY "ISO10646"
CHARSET_ENCODING "1"
ENDPROPERTIES
CHARS 2
STARTCHAR A
ENCODING 65
SWIDTH 1000 0
DWIDTH 8 0
BBX 8 8 0 -2
BITMAP
00
18
24
42
7E
42
42
00
ENDCHAR
STARTCHAR uni4E2D
ENCODING 20013
SWIDTH 1000 0
DWIDTH 8 0
BBX 8 8 0 -2
BITMAP
10
7C
54
54
7C
10
10
00
ENDCHAR
ENDFONT
`
async function setup(page: Page) {
  const directory = mkdtempSync(join(tmpdir(), 'fontcarver-p3-e2e-')),
    path = join(directory, 'BitmapTest.bdf')
  writeFileSync(path, sample)
  await page.goto('/')
  const doc = await openPathThroughDialog(page, path)
  await menuItem(page, 'Bitmap', /^Bitmap editor$/)
  await expect(
    page.getByRole('heading', { name: 'A 8px · 1bpp' }),
  ).toBeVisible()
  await expect(page.getByLabel('Bitmap pixel canvas')).toBeVisible()
  return { directory, doc }
}
/**
 * The document's own tab. An unlabelled Overview renders as
 * `Overview · <file>`, so the file name is matched at the start of the tab
 * name or after that separator rather than as the whole name.
 */
const FONT_TAB = /(?:^|· )BitmapTest\.bdf$/
const image = (page: Page, doc: number, ppem = 8, depth = 1) =>
  api<BitmapGlyphView>(page, 'bitmap.glyph', { doc, ppem, depth, glyph: 'A' })
async function pixel(page: Page, x: number, y: number) {
  const canvas = page.getByLabel('Bitmap pixel canvas'),
    b = await canvas.boundingBox()
  if (!b) throw new Error('missing canvas')
  await canvas.click({
    // The guide frame adds one pixel around this 8×8 bitmap.
    position: { x: ((x + 1.5) * b.width) / 10, y: ((y + 1.5) * b.height) / 10 },
  })
}
test('BDF pixels survive drawing, undo, redo, save, export and reopen', async ({
  page,
}) => {
  const { directory, doc } = await setup(page),
    original = await image(page, doc.doc)
  await pixel(page, 0, 0)
  await expect
    .poll(async () => (await image(page, doc.doc)).value?.bitmap.rows[0])
    .toBe(128)
  await menuItem(page, 'Edit', /^Undo/)
  await expect
    .poll(async () => (await image(page, doc.doc)).value)
    .toEqual(original.value)
  await menuItem(page, 'Edit', /^Redo/)
  await expect
    .poll(async () => (await image(page, doc.doc)).value?.bitmap.rows[0])
    .toBe(128)
  const native = join(directory, 'Edited.fcf')
  await page.keyboard.press('Control+s')
  const save = page.getByRole('dialog', { name: 'Save As' })
  await save.getByRole('textbox', { name: 'Document path' }).fill(native)
  await save.getByRole('button', { name: 'Save', exact: true }).click()
  await expect(save).toBeHidden()
  await menuItem(page, 'File', /^Export/)
  const dialog = page.getByRole('dialog', { name: 'Export' })
  await dialog.getByRole('tab', { name: 'BDF', exact: true }).click()
  const output = join(directory, 'Edited.bdf')
  await dialog.getByLabel('Destination path').fill(output)
  await dialog.getByRole('button', { name: 'Export', exact: true }).click()
  await expect(
    dialog.getByRole('button', { name: 'Export again' }),
  ).toBeVisible()
  await dialog.getByRole('button', { name: 'Close', exact: true }).click()
  expect(readFileSync(output, 'utf8')).toContain('80\n18\n24')
  const reopened = await openPathThroughDialog(page, native)
  expect((await image(page, reopened.doc)).value?.bitmap.rows[0]).toBe(128)
})
test('failed pixel writes retain the draft and block tab navigation until retry', async ({
  page,
}) => {
  const { doc } = await setup(page)
  await page.route('**/api/bitmap.apply', (route) =>
    route.fulfill({
      status: 409,
      contentType: 'application/json',
      body: JSON.stringify({
        error: { code: 'invalid_params', message: 'Injected conflict' },
      }),
    }),
  )
  await pixel(page, 0, 0)
  await expect(
    page.getByRole('alert').filter({ hasText: 'local pixels' }),
  ).toBeVisible()
  await page.getByRole('tab', { name: FONT_TAB }).click()
  await expect(
    page.getByRole('tab', { name: 'Bitmap editor', exact: true }),
  ).toHaveAttribute('aria-selected', 'true')
  expect((await image(page, doc.doc)).value?.bitmap.rows[0]).toBe(0)
  await page.unroute('**/api/bitmap.apply')
  await page.getByRole('button', { name: 'Retry', exact: true }).click()
  await expect
    .poll(async () => (await image(page, doc.doc)).value?.bitmap.rows[0])
    .toBe(128)
  await page.getByRole('tab', { name: FONT_TAB }).click()
  await expect(page.getByRole('tab', { name: FONT_TAB })).toHaveAttribute(
    'aria-selected',
    'true',
  )
})
test('trace, regenerate, strike properties, tools and strike removal are undoable', async ({
  page,
}) => {
  const { doc } = await setup(page)
  await page
    .getByRole('button', {
      name: 'Replace outline with pixel trace',
      exact: true,
    })
    .click()
  await page.getByRole('spinbutton', { name: 'Size (ppem)' }).fill('16')
  await page
    .getByRole('button', { name: 'Rasterize outlines', exact: true })
    .click()
  await expect
    .poll(
      async () =>
        (await api<BitmapState>(page, 'bitmap.state', { doc: doc.doc })).strikes
          .length,
    )
    .toBe(2)
  await page
    .getByRole('combobox', { name: 'Strike', exact: true })
    .selectOption('8:1')
  const before = await image(page, doc.doc)
  await page
    .getByRole('button', { name: 'Flip horizontally', exact: true })
    .click()
  await expect
    .poll(async () => (await image(page, doc.doc)).generation)
    .toBeGreaterThan(before.generation)
  await page.getByRole('button', { name: 'Copy pixels', exact: true }).click()
  await page.getByRole('button', { name: 'Cut pixels', exact: true }).click()
  await expect
    .poll(async () =>
      (await image(page, doc.doc)).value?.bitmap.rows.every((n) => n === 0),
    )
    .toBe(true)
  await page.getByRole('button', { name: 'Paste pixels', exact: true }).click()
  await expect
    .poll(async () =>
      (await image(page, doc.doc)).value?.bitmap.rows.some((n) => n !== 0),
    )
    .toBe(true)
  await page
    .getByText('Strike metrics and BDF properties', { exact: true })
    .click()
  await page.getByLabel('Property', { exact: true }).fill('FOUNDRY')
  await page.getByLabel('Value', { exact: true }).fill('FontCarver')
  await page.getByRole('button', { name: 'Set property', exact: true }).click()
  await expect
    .poll(
      async () =>
        (await api<BitmapState>(page, 'bitmap.state', { doc: doc.doc }))
          .strikes[0]?.properties.FOUNDRY,
    )
    .toEqual({ Str: 'FontCarver' })
  await page.getByRole('button', { name: 'Delete strike', exact: true }).click()
  await expect
    .poll(
      async () =>
        (await api<BitmapState>(page, 'bitmap.state', { doc: doc.doc })).strikes
          .length,
    )
    .toBe(1)
  await menuItem(page, 'Edit', /^Undo/)
  await expect
    .poll(
      async () =>
        (await api<BitmapState>(page, 'bitmap.state', { doc: doc.doc })).strikes
          .length,
    )
    .toBe(2)
  await page.getByLabel('Waterfall sample').fill('A中')
  await expect(page.locator('canvas')).toHaveCount(3)
})

test('focused metrics are saved, and ellipse, crop, invert and scaling retain pixels', async ({
  page,
}) => {
  const { doc, directory } = await setup(page)
  await page
    .getByRole('spinbutton', { name: 'Advance', exact: true })
    .fill('11')
  await page.keyboard.press('Control+s')
  const save = page.getByRole('dialog', { name: 'Save As' }),
    path = join(directory, 'Metrics.fcf')
  await save.getByRole('textbox', { name: 'Document path' }).fill(path)
  await save.getByRole('button', { name: 'Save', exact: true }).click()
  await expect(save).toBeHidden()
  expect((await image(page, doc.doc)).value?.advance).toBe(11)
  await page
    .getByText('Strike metrics and BDF properties', { exact: true })
    .click()
  await page.getByRole('spinbutton', { name: 'Ascent', exact: true }).fill('7')
  await page.keyboard.press('Control+s')
  await expect
    .poll(
      async () =>
        (await api<BitmapState>(page, 'bitmap.state', { doc: doc.doc }))
          .strikes[0]?.ascent,
    )
    .toBe(7)
  await page
    .getByText('Strike metrics and BDF properties', { exact: true })
    .click()
  await page.getByRole('button', { name: 'Ellipse', exact: true }).click()
  const canvas = page.getByLabel('Bitmap pixel canvas'),
    box = await canvas.boundingBox()
  if (!box) throw new Error('canvas missing')
  await page.mouse.move(box.x + 24, box.y + 24)
  await page.mouse.down()
  await page.mouse.move(box.x + 104, box.y + 104, { steps: 6 })
  await page.mouse.up()
  const before = (await image(page, doc.doc)).value?.bitmap.rows
  await page.getByRole('button', { name: 'Invert', exact: true }).click()
  await expect
    .poll(async () => (await image(page, doc.doc)).value?.bitmap.rows)
    .not.toEqual(before)
  await page.getByText('Scale pixels', { exact: true }).first().click()
  await page.getByLabel('Target width').fill('16')
  await page.getByLabel('Target height').fill('16')
  await page.getByRole('button', { name: 'Scale pixels', exact: true }).click()
  await expect
    .poll(async () => (await image(page, doc.doc)).value?.bitmap.width)
    .toBe(16)
  await page
    .getByRole('button', { name: 'Crop to selection / ink', exact: true })
    .click()
  await expect(canvas).toBeVisible()
})
