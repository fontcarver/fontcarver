import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { expect, type Page } from '@playwright/test'
import { test } from './sessionFixtures'
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
  // The isolated session contains only this workflow's bitmap document.
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
  await page.getByRole('tab', { name: FONT_TAB }).last().click()
  await expect(
    page.getByRole('tab', { name: FONT_TAB }).last(),
  ).toHaveAttribute('aria-selected', 'true')
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

test('pixel tools, the marquee copy and the persisted overlays', async ({
  page,
}) => {
  const { doc } = await setup(page)
  const pressed = (name: string) =>
    page.getByRole('button', { name, exact: true })
  // Every tool is a command, so the Bitmap menu and the command palette select
  // the same tool the toolbar shows; the single-key bindings have their own
  // spec at the end of this file. A menu item's accessible name carries its
  // shortcut, so each pattern matches the label as a prefix.
  for (const [item, name] of [
    [/^Select/, 'Select'],
    [/^Eyedropper/, 'Eyedropper'],
    [/^Fill/, 'Fill'],
    [/^Ellipse/, 'Ellipse'],
    [/^Pencil/, 'Pencil'],
  ] as const) {
    await menuItem(page, 'Bitmap', item)
    await expect(pressed(name)).toHaveAttribute('aria-pressed', 'true')
  }
  // Alt-drag over a marquee stamps a copy instead of moving the pixels.
  await menuItem(page, 'Bitmap', /^Select/)
  const canvas = page.getByLabel('Bitmap pixel canvas')
  const box = await canvas.boundingBox()
  if (!box) throw new Error('canvas missing')
  // The guide frame adds one pixel around this 8x8 bitmap in both axes.
  const at = (x: number, y: number) => ({
    x: box.x + ((x + 1.5) * box.width) / 10,
    y: box.y + ((y + 1.5) * box.height) / 10,
  })
  const before = (await image(page, doc.doc)).value?.bitmap.rows
  const start = at(1, 1)
  await page.mouse.move(start.x, start.y)
  await page.mouse.down()
  await page.mouse.move(at(6, 1).x, at(6, 1).y, { steps: 4 })
  await page.mouse.up()
  await page.mouse.move(start.x, start.y)
  await page.keyboard.down('Alt')
  await page.mouse.down()
  await page.mouse.move(at(1, 5).x, at(1, 5).y, { steps: 4 })
  await page.mouse.up()
  await page.keyboard.up('Alt')
  await expect
    .poll(async () => (await image(page, doc.doc)).value?.bitmap.rows)
    .not.toEqual(before)
  const copied = (await image(page, doc.doc)).value?.bitmap.rows ?? []
  // Row 1 keeps its original ink and row 5 now carries the same bits.
  expect(copied[1]).toBe(before?.[1])
  expect(copied[5]).toBe(before?.[1])
  // The overlay switches are commands too, and each choice is remembered.
  await page.getByRole('checkbox', { name: 'Advance column' }).uncheck()
  await menuItem(page, 'Bitmap', /^Font units/)
  await expect(
    page.getByRole('checkbox', { name: 'Font units' }),
  ).not.toBeChecked()
  expect(
    await page.evaluate(() =>
      JSON.parse(localStorage.getItem('fontcarver.bitmapView.v1') ?? '{}'),
    ),
  ).toMatchObject({ advanceColumn: false, fontUnits: false })
  // Leaving the editor and coming back keeps both switches where they were.
  await page.getByRole('tab', { name: FONT_TAB }).last().click()
  await page.getByRole('tab', { name: 'Bitmap editor', exact: true }).click()
  await expect(page.getByLabel('Bitmap pixel canvas')).toBeVisible()
  await expect(
    page.getByRole('checkbox', { name: 'Advance column' }),
  ).not.toBeChecked()
  await page.getByRole('checkbox', { name: 'Font units' }).check()
  await expect(page.getByLabel('Bitmap pointer coordinates')).toBeVisible()
})

test('an exact grid preset sets the strike metrics and a custom size is corrected', async ({
  page,
}) => {
  const { doc } = await setup(page)
  const size = page.getByRole('spinbutton', { name: 'Size (ppem)' })
  await size.fill('17')
  const hint = page.getByText(/is not a whole number/)
  await expect(hint).toBeVisible()
  const suggestion = page.getByRole('button', { name: /Use \d+ ppem instead/ })
  await expect(suggestion).toBeVisible()
  const label = (await suggestion.textContent()) ?? ''
  const nearest = Number(/\d+/.exec(label)?.[0])
  const grid = await api<{
    presets: { ppem: number; ascent: number; descent: number }[]
  }>(page, 'bitmap.grid', { doc: doc.doc, master: null, ppem: nearest })
  const expected = grid.presets.find((preset) => preset.ppem === nearest)
  expect(expected).toBeDefined()
  await suggestion.click()
  await expect(size).toHaveValue(String(nearest))
  await page
    .getByRole('button', { name: 'Create empty strike', exact: true })
    .click()
  await expect
    .poll(async () =>
      (
        await api<BitmapState>(page, 'bitmap.state', { doc: doc.doc })
      ).strikes.find((strike) => strike.ppem === nearest),
    )
    .toMatchObject({
      ppem: nearest,
      depth: 1,
      ascent: expected!.ascent,
      descent: expected!.descent,
    })
})

test('properties are renamed and a sprite sheet round trips through its own layout', async ({
  page,
}) => {
  const { doc, directory } = await setup(page)
  await page
    .getByText('Strike metrics and BDF properties', { exact: true })
    .click()
  await page.getByLabel('Property', { exact: true }).fill('MY_TAG')
  await page.getByLabel('Value', { exact: true }).fill('one')
  await page.getByRole('button', { name: 'Set property', exact: true }).click()
  await expect
    .poll(
      async () =>
        (await api<BitmapState>(page, 'bitmap.state', { doc: doc.doc }))
          .strikes[0]?.properties.MY_TAG,
    )
    .toEqual({ Str: 'one' })
  await page.getByRole('button', { name: 'Rename MY_TAG' }).click()
  await page.getByLabel('New name for MY_TAG').fill('MY_OTHER_TAG')
  await page.getByRole('button', { name: 'Save name', exact: true }).click()
  await expect
    .poll(async () => {
      const properties = (
        await api<BitmapState>(page, 'bitmap.state', { doc: doc.doc })
      ).strikes[0]?.properties
      return [properties?.MY_TAG, properties?.MY_OTHER_TAG]
    })
    .toEqual([undefined, { Str: 'one' }])
  // The sheet export carries its layout, so the import needs no retyping.
  const original = await image(page, doc.doc)
  await page.getByText('PNG sprite sheet', { exact: true }).click()
  await page.getByLabel('Glyph names in cell order').fill('A uni4E2D')
  await page.getByLabel('Cell width', { exact: true }).fill('8')
  await page.getByLabel('Cell height', { exact: true }).fill('8')
  await page.getByLabel('Columns', { exact: true }).fill('2')
  await page
    .getByRole('button', { name: 'Prepare PNG export', exact: true })
    .click()
  const save = page.getByRole('button', { name: 'Save PNG…' })
  await expect(save).toBeVisible()
  await expect(page.getByLabel('Sheet layout')).toHaveValue(
    /cells 8x8\ncolumns 2\nglyphs 2\nA uni4E2D/,
  )
  const download = await Promise.all([
    page.waitForEvent('download'),
    save.click(),
  ]).then(([event]) => event)
  const sheet = join(directory, 'sheet.png')
  await download.saveAs(sheet)
  // Clearing the form proves the PNG alone carries the mapping.
  await page.getByLabel('Glyph names in cell order').fill('A')
  await page.getByLabel('Columns', { exact: true }).fill('1')
  await page.getByLabel('PNG file').setInputFiles(sheet)
  await page
    .getByRole('button', { name: 'Import PNG into strike', exact: true })
    .click()
  await expect(page.getByText('Imported 2 glyphs.')).toBeVisible()
  await expect
    .poll(async () => (await image(page, doc.doc)).value?.bitmap.rows)
    .toEqual(original.value?.bitmap.rows)
  await expect(page.getByLabel('Glyph names in cell order')).toHaveValue(
    'A uni4E2D',
  )
})

test('cross-strike copies, batch tracing and the single-glyph waterfall', async ({
  page,
}) => {
  const { doc } = await setup(page)
  const transfer = page
    .locator('details')
    .filter({ hasText: 'Copy between strikes' })
  await page.getByText('Copy between strikes', { exact: true }).click()
  await transfer.getByLabel('Target size (ppem)').fill('4')
  await transfer.getByLabel('Target depth').selectOption('8')
  await transfer.getByLabel('Scale filter').selectOption('AreaAverage')
  await transfer.getByLabel('All images in this strike').check()
  await transfer
    .getByRole('button', { name: 'Copy to target strike', exact: true })
    .click()
  await expect
    .poll(async () => {
      const state = await api<BitmapState>(page, 'bitmap.state', {
        doc: doc.doc,
      })
      return state.strikes.find((s) => s.ppem === 4 && s.depth === 8)
        ?.glyph_count
    })
    .toBe(2)
  // Area averaging produces partial coverage, which 1 bpp could not hold.
  const scaled = await image(page, doc.doc, 4, 8)
  expect(
    scaled.value?.bitmap.rows.some((value) => value > 0 && value < 255),
  ).toBe(true)
  // Every image in the strike becomes an outline in one undo step.
  await page
    .getByRole('combobox', { name: 'Strike', exact: true })
    .selectOption('8:1')
  await page.getByLabel('Every image in this strike (2)').check()
  await page
    .getByRole('button', {
      name: 'Replace outline with pixel trace',
      exact: true,
    })
    .click()
  const list = await api<{ items: { gid: number; name: string }[] }>(
    page,
    'glyphs.list',
    {
      doc: doc.doc,
      master: null,
      filter: {
        query: null,
        query_scope: 'All',
        case_sensitive: false,
        regex: false,
        smart_expression: null,
        glyph_names: null,
        cjk: null,
        categories: [],
        scripts: [],
        blocks: [],
        unencoded_only: false,
        color_labels: [],
        tags: [],
        unicode_ranges: [],
        has_components: null,
        changed_only: false,
      },
      sort: 'Order',
      offset: 0,
      limit: 20,
      include_paths: false,
    },
  )
  const gid = list.items.find((item) => item.name === 'uni4E2D')?.gid
  expect(gid).toBeDefined()
  const traced = async () =>
    (
      await api<{ glyph: { layers: { contours: unknown[] }[] } }>(
        page,
        'glyphs.detail',
        { doc: doc.doc, gid },
      )
    ).glyph.layers[0]?.contours.length
  await expect.poll(traced).toBeGreaterThan(0)
  await menuItem(page, 'Edit', /^Undo/)
  await expect.poll(traced).toBe(0)
  // The waterfall can show the selected glyph at every size.
  const toggle = page.getByRole('checkbox', { name: /^Show only A at every/ })
  await toggle.check()
  // The strike selector and the editor heading show the same sizes, so match
  // inside the waterfall section itself.
  const waterfall = page
    .locator('section')
    .filter({ has: page.getByLabel('Waterfall sample') })
  await expect(waterfall.getByText('4px · 8bpp', { exact: true })).toBeVisible()
  await expect(waterfall.getByText('8px · 1bpp', { exact: true })).toBeVisible()
})

test('positions upright bitmap proof pixels using vertical origins and advances', async ({
  page,
}) => {
  const { doc } = await setup(page)
  const state = await api<BitmapState>(page, 'bitmap.state', { doc: doc.doc })
  const glyph = (vertical_origin: [number, number]) => ({
    bitmap: {
      width: 3,
      height: 4,
      depth: 1,
      origin_x: -1,
      origin_y: -2,
      rows: [224, 224, 224, 224],
    },
    advance: 8,
    vertical_advance: 12,
    vertical_origin,
    refs: [],
  })
  await api(page, 'bitmap.apply', {
    doc: doc.doc,
    expected_generation: state.generation,
    label: 'Prepare vertical bitmap proof',
    command: {
      SetStrike: {
        strike: {
          ppem: 10,
          depth: 1,
          ascent: 8,
          descent: 2,
          properties: {},
          glyphs: { A: glyph([2, 7]), uni4E2D: glyph([4, 9]) },
        },
      },
    },
  })
  await page.getByLabel('Waterfall sample').fill('A中')
  const proof = page.getByRole('img', { name: 'Bitmap proof at 10 px, 1 bpp' })
  await expect(proof).toBeVisible()
  await page
    .getByRole('checkbox', { name: 'Vertical writing', exact: true })
    .check()
  const pixels = () =>
    proof.evaluate((element) => {
      const canvas = element as unknown as {
        width: number
        height: number
        getContext: (kind: '2d') => {
          getImageData: (
            x: number,
            y: number,
            width: number,
            height: number,
          ) => { data: ArrayLike<number> }
        } | null
      }
      const data = canvas
        .getContext('2d')
        ?.getImageData(0, 0, canvas.width, canvas.height).data
      if (!data) throw new Error('bitmap proof canvas has no pixels')
      const ink = []
      for (let y = 0; y < canvas.height; y++)
        for (let x = 0; x < canvas.width; x++) {
          const index = (y * canvas.width + x) * 4
          if (
            data[index]! < 100 &&
            data[index + 1]! < 100 &&
            data[index + 2]! < 100
          )
            ink.push([x, y])
        }
      return { width: canvas.width, height: canvas.height, ink }
    })
  const expectedInk = [
    [2, 15],
    [0, 29],
  ].flatMap(([left, top]) =>
    Array.from({ length: 4 }, (_, y) =>
      Array.from({ length: 3 }, (_, x) => [left! + x, top! + y]),
    ).flat(),
  )
  await expect.poll(pixels).toEqual({ width: 10, height: 44, ink: expectedInk })
})

test('every pixel tool has a working single-key shortcut', async ({ page }) => {
  await setup(page)
  const pressed = (name: string) =>
    page.getByRole('button', { name, exact: true })
  for (const [key, name] of [
    ['m', 'Select'],
    ['i', 'Eyedropper'],
    ['g', 'Fill'],
    ['l', 'Line'],
    ['r', 'Rectangle'],
    ['o', 'Ellipse'],
    ['e', 'Eraser'],
    ['h', 'Hand'],
    ['b', 'Pencil'],
  ] as const) {
    await page.keyboard.press(key)
    await expect(pressed(name)).toHaveAttribute('aria-pressed', 'true')
  }
  // The same keys do nothing while a text field has focus.
  await page.getByLabel('Waterfall sample').click()
  await page.keyboard.press('m')
  await expect(pressed('Pencil')).toHaveAttribute('aria-pressed', 'true')
})
