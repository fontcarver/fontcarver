import { expect, test } from '@playwright/test'

import { api } from './support'
import { docs, firstOnCurve, outline } from './acceptanceSupport'

const TEXT = 'Hamburgefonstiv'

interface RunGlyph {
  documentGid: number | null
  name: string | null
  x: number
  y: number
  xAdvance: number
  yAdvance: number
}

interface ShapedRun {
  glyphs: {
    gid: number
    x_advance: number
    y_advance: number
    x_offset: number
    y_offset: number
    cluster: number
  }[]
}

async function runGlyphs(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    const run = (
      globalThis as unknown as {
        __fwTextRun?: { glyphs: RunGlyph[]; activeIndex: number | null } | null
      }
    ).__fwTextRun
    if (!run) throw new Error('missing text-run dev handle')
    return { glyphs: run.glyphs, activeIndex: run.activeIndex }
  })
}

async function editorState(page: import('@playwright/test').Page) {
  return page.evaluate(() => {
    const editor = (
      globalThis as unknown as {
        __fwEditor?: {
          currentSelection: { size: number }
          viewportForTest: {
            scale: number
            toScreen: (point: { x: number; y: number }) => {
              x: number
              y: number
            }
          }
        }
      }
    ).__fwEditor
    if (!editor) throw new Error('missing editor dev handle')
    return {
      selectionSize: editor.currentSelection.size,
      scale: editor.viewportForTest.scale,
    }
  })
}

async function moveLocalPoint(
  page: import('@playwright/test').Page,
  doc: number,
  gid: number,
  location: { contour: number; point: number; x: number; y: number },
  delta: { x: number; y: number },
) {
  const canvas = page.getByRole('img', { name: /^Glyph / })
  const box = await canvas.boundingBox()
  if (!box) throw new Error('glyph canvas has no bounds')
  const screen = await page.evaluate(({ x, y }) => {
    const editor = (
      globalThis as unknown as {
        __fwEditor?: {
          viewportForTest: {
            scale: number
            toScreen: (point: { x: number; y: number }) => {
              x: number
              y: number
            }
          }
        }
      }
    ).__fwEditor
    if (!editor) throw new Error('missing editor dev handle')
    return {
      ...editor.viewportForTest.toScreen({ x, y }),
      scale: editor.viewportForTest.scale,
    }
  }, location)
  const start = { x: box.x + screen.x, y: box.y + screen.y }
  const end = {
    x: start.x + delta.x * screen.scale,
    y: start.y - delta.y * screen.scale,
  }
  await page.mouse.move(start.x, start.y)
  await page.keyboard.down('Control')
  await page.mouse.down()
  await page.mouse.move(end.x, end.y)
  await page.mouse.up()
  await page.keyboard.up('Control')

  let moved = { x: Number.NaN, y: Number.NaN }
  await expect
    .poll(async () => {
      const point = (await outline(page, doc, gid)).contours[location.contour]
        ?.points[location.point]
      if (point) moved = point
      return (
        !!point &&
        Math.abs(point.x - location.x - delta.x) < 0.01 &&
        Math.abs(point.y - location.y - delta.y) < 0.01
      )
    })
    .toBe(true)
  return { ...location, ...moved }
}

test('shapes text and edits the third glyph in local coordinates', async ({
  page,
}) => {
  test.setTimeout(120_000)
  await page.goto('/')
  const [document] = await docs(page)
  if (!document) throw new Error('DejaVu fixture was not opened by the server')

  const grid = page.getByRole('listbox', { name: 'Glyphs' })
  const a = grid.getByRole('option', { name: 'A', exact: true })
  await expect(a).toBeVisible()
  await a.dblclick()
  const canvas = page.getByRole('img', { name: /^Glyph A/ })
  await expect(canvas).toBeVisible()

  await canvas.press('t')
  const view = page.locator('[data-text-mode]')
  await expect(view).toHaveAttribute('data-text-mode', 'text')
  const input = page.getByRole('textbox', { name: 'Text', exact: true })
  await expect(input).toBeFocused()
  await input.fill(TEXT)
  await expect(view).toHaveAttribute('data-run-count', String(TEXT.length))
  await expect
    .poll(async () => (await runGlyphs(page)).glyphs.length)
    .toBe(TEXT.length)

  const shaped = await api<ShapedRun>(page, 'text.shape', {
    doc: document.doc,
    text: TEXT,
    master: null,
    features: [
      ['kern', true],
      ['liga', true],
      ['calt', true],
    ],
    script: null,
    language: null,
    direction: 'ltr',
    size: null,
  })
  const positioned = (await runGlyphs(page)).glyphs
  const first = positioned[0]
  const second = positioned[1]
  const firstShaped = shaped.glyphs[0]
  const secondShaped = shaped.glyphs[1]
  if (!first || !second || !firstShaped || !secondShaped)
    throw new Error('shaping did not produce the first two glyphs')
  expect(first.x).toBe(firstShaped.x_offset)
  expect(first.y).toBe(firstShaped.y_offset)
  expect(first.xAdvance).toBe(firstShaped.x_advance)
  expect(first.yAdvance).toBe(firstShaped.y_advance)
  expect(second.x).toBe(firstShaped.x_advance + secondShaped.x_offset)
  expect(second.y).toBe(firstShaped.y_advance + secondShaped.y_offset)
  expect(second.x).toBe(first.x + first.xAdvance)

  const target = await canvas.evaluate((element) => {
    const globals = globalThis as unknown as {
      __fwTextRun?: { glyphs: RunGlyph[] } | null
      __fwEditor?: {
        viewportForTest: {
          toScreen: (point: { x: number; y: number }) => {
            x: number
            y: number
          }
        }
      }
    }
    const glyph = globals.__fwTextRun?.glyphs[2]
    const editor = globals.__fwEditor
    if (!glyph || !editor) throw new Error('third shaped glyph is unavailable')
    const rect = (
      element as unknown as {
        getBoundingClientRect: () => { left: number; top: number }
      }
    ).getBoundingClientRect()
    const at = editor.viewportForTest.toScreen({
      x: glyph.x + glyph.xAdvance / 2,
      y: glyph.y + 300,
    })
    return { x: rect.left + at.x, y: rect.top + at.y }
  })
  await page.mouse.click(target.x, target.y)
  await expect(view).toHaveAttribute('data-text-mode', 'edit')
  await expect.poll(async () => (await runGlyphs(page)).activeIndex).toBe(2)
  const third = (await runGlyphs(page)).glyphs[2]
  if (!third?.name || third.documentGid === null)
    throw new Error('third glyph did not resolve to a document glyph')
  expect(third.name).toBe('m')
  await expect(
    page.getByRole('img', { name: new RegExp(`^Glyph ${third.name}`) }),
  ).toBeVisible()

  const originalOutline = await outline(page, document.doc, third.documentGid)
  const originalPoint = firstOnCurve(originalOutline)
  const movedPoint = await moveLocalPoint(
    page,
    document.doc,
    third.documentGid,
    originalPoint,
    { x: 8, y: 5 },
  )
  expect(movedPoint.x).toBeCloseTo(originalPoint.x + 8, 2)
  expect(movedPoint.y).toBeCloseTo(originalPoint.y + 5, 2)
  expect((await editorState(page)).selectionSize).toBeGreaterThan(0)

  const activeCanvas = page.getByRole('img', { name: /^Glyph m/ })
  await activeCanvas.press('Control+ArrowRight')
  await activeCanvas.press('Control+ArrowRight')
  await activeCanvas.press('Control+ArrowRight')
  await page.waitForTimeout(650)
  await expect
    .poll(
      async () => (await outline(page, document.doc, third.documentGid!)).width,
    )
    .toBe(originalOutline.width + 3)

  await activeCanvas.press('Control+z')
  await expect
    .poll(
      async () => (await outline(page, document.doc, third.documentGid!)).width,
    )
    .toBe(originalOutline.width)
  const afterMetricsUndo = await outline(page, document.doc, third.documentGid)
  const stillMoved =
    afterMetricsUndo.contours[movedPoint.contour]?.points[movedPoint.point]
  expect(stillMoved?.x).toBeCloseTo(movedPoint.x, 5)
  expect(stillMoved?.y).toBeCloseTo(movedPoint.y, 5)
  await activeCanvas.press('Control+z')
  await expect
    .poll(async () => {
      const restored = (await outline(page, document.doc, third.documentGid!))
        .contours[originalPoint.contour]?.points[originalPoint.point]
      return (
        !!restored &&
        Math.abs(restored.x - originalPoint.x) < 0.01 &&
        Math.abs(restored.y - originalPoint.y) < 0.01
      )
    })
    .toBe(true)

  const initialScale = (await editorState(page)).scale
  const size = page.getByRole('textbox', {
    name: 'Preview size',
    exact: true,
  })
  await expect(size).toHaveValue('128')
  await size.fill('256')
  await size.press('Enter')
  await expect
    .poll(async () => (await editorState(page)).scale)
    .toBeCloseTo(256 / 2048, 6)
  expect((await editorState(page)).scale).not.toBe(initialScale)

  await activeCanvas.press('Escape')
  await expect.poll(async () => (await editorState(page)).selectionSize).toBe(0)
  await expect(view).toHaveAttribute('data-text-mode', 'edit')
  await activeCanvas.press('Escape')
  await expect(view).toHaveAttribute('data-text-mode', 'text')
  await expect(input).toBeFocused()
})
