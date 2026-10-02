import { expect, type Page } from '@playwright/test'

import type { DocumentHistory } from '../src/api/generated/DocumentHistory'
import {
  firstOnCurve,
  glyphCell,
  menuItem,
  openGlyphFromGrid,
  outline,
} from './acceptanceSupport'
import { test } from './p2aSupport'
import { api } from './support'

async function pointPosition(page: Page, point: { x: number; y: number }) {
  const canvas = page.getByRole('img', { name: /^Glyph / })
  const bounds = await canvas.boundingBox()
  if (!bounds) throw new Error('glyph canvas has no bounds')
  const local = await page.evaluate((at) => {
    const editor = (
      globalThis as unknown as {
        __fwEditor: {
          viewportForTest: {
            toScreen: (point: { x: number; y: number }) => {
              x: number
              y: number
            }
          }
        }
      }
    ).__fwEditor
    return editor.viewportForTest.toScreen(at)
  }, point)
  return { x: bounds.x + local.x, y: bounds.y + local.y }
}

test('keeps the laptop canvas usable and reaches every tool with the keyboard', async ({
  page,
  source,
}) => {
  expect(source.doc.doc).toBeGreaterThan(0)
  await page.setViewportSize({ width: 1366, height: 768 })
  await openGlyphFromGrid(page, 'A')
  const canvas = page.getByRole('img', { name: /^Glyph / })
  await expect(
    page.getByRole('button', { name: 'Kerning', exact: true }),
  ).toHaveAttribute('aria-expanded', 'false')
  await expect(page.getByTestId('text-kerning-panel')).toBeHidden()
  const bounds = await canvas.boundingBox()
  expect(bounds?.height).toBeGreaterThanOrEqual(300)

  const preview = page.getByRole('region', { name: 'Preview', exact: true })
  const divider = page.getByRole('separator', { name: 'Resize preview' })
  const height = (await preview.boundingBox())?.height
  await expect(divider).toHaveAttribute('aria-valuenow', '96')
  const settings = preview.getByRole('button', { name: 'Preview settings' })
  await settings.click()
  expect((await preview.boundingBox())?.height).toBeGreaterThanOrEqual(160)
  await settings.click()
  expect((await preview.boundingBox())?.height).toBe(height)
  await preview.getByRole('button', { name: 'Collapse preview' }).click()
  await expect(divider).toBeHidden()
  await preview.getByRole('button', { name: 'Expand preview' }).click()
  await expect(divider).toBeVisible()

  const toolbar = page.getByRole('toolbar', { name: 'Tools', exact: true })
  const buttons = toolbar.getByRole('button')
  await buttons.first().focus()
  for (let index = 0; index < (await buttons.count()); index++) {
    const button = buttons.nth(index)
    await expect(button).toBeFocused()
    const rect = await button.boundingBox()
    const strip = await toolbar.boundingBox()
    if (!rect || !strip) throw new Error('tool strip has no bounds')
    expect(rect.y).toBeGreaterThanOrEqual(strip.y - 1)
    expect(rect.y + rect.height).toBeLessThanOrEqual(strip.y + strip.height + 1)
    expect(rect.y + rect.height).toBeLessThanOrEqual(768)
    await page.keyboard.press('ArrowDown')
  }
  await expect(buttons.first()).toBeFocused()
  await page.keyboard.press('End')
  await expect(
    toolbar.getByRole('button', { name: 'Pixel', exact: true }),
  ).toBeFocused()
  await page.keyboard.press('Home')
  await expect(buttons.first()).toBeFocused()
  await page.keyboard.press('Tab')
  await expect(buttons.first()).not.toBeFocused()

  await expect(
    page.getByRole('textbox', { name: 'Custom glyph data JSON' }),
  ).toBeHidden()
  await page.getByText('Glyph database', { exact: true }).click()
  await page.getByText('Custom glyph data', { exact: true }).click()
  await expect(
    page.getByRole('textbox', { name: 'Custom glyph data JSON' }),
  ).toBeVisible()
})

test('customizes one tool binding and preserves hold release and menu focus', async ({
  page,
  source,
}) => {
  expect(source.doc.doc).toBeGreaterThan(0)
  await openGlyphFromGrid(page, 'A')
  const canvas = page.getByRole('img', { name: /^Glyph / })
  await menuItem(page, 'Help', /^Keyboard Shortcuts/)
  const dialog = page.getByRole('dialog', { name: 'Keyboard Shortcuts' })
  await dialog.getByRole('searchbox', { name: 'Search commands' }).fill('Pen')
  const binding = dialog.getByRole('textbox', {
    name: 'Shortcut for Pen',
    exact: true,
  })
  await expect(binding).toHaveCount(1)
  await binding.fill('U')
  await dialog.getByRole('button', { name: 'Apply', exact: true }).click()
  await expect(dialog).toBeHidden()
  const tools = page.getByRole('toolbar', { name: 'Tools', exact: true })
  const select = tools.getByRole('button', { name: 'Select', exact: true })
  const pen = tools.getByRole('button', { name: 'Pen', exact: true })
  await select.click()
  await canvas.focus()
  await page.keyboard.down('u')
  await expect(pen).toHaveAttribute('aria-pressed', 'true')
  // The documented hold transition occurs after 200 ms; release must restore
  // Select rather than leaving the global command's Pen selection active.
  await page.waitForTimeout(250)
  await page.keyboard.up('u')
  await expect(select).toHaveAttribute('aria-pressed', 'true')
  await canvas.press('u')
  await expect(pen).toHaveAttribute('aria-pressed', 'true')

  const text = page.getByRole('textbox', { name: 'Text', exact: true })
  await text.fill('A')
  await text.press('u')
  await expect(text).toHaveValue('Au')
  await expect(pen).toHaveAttribute('aria-pressed', 'true')

  await canvas.focus()
  const view = page.getByRole('menuitem', { name: 'View', exact: true })
  await view.click()
  await view.press('ArrowDown')
  await expect(
    page.getByRole('menu', { name: 'View' }).getByRole('menuitem').first(),
  ).toBeFocused()
  await page.keyboard.press('Escape')
  await expect(view).toBeFocused()
  await expect(page.getByRole('menu', { name: 'View' })).toBeHidden()
  await view.press('Enter')
  await expect(
    page.getByRole('menu', { name: 'View' }).getByRole('menuitem').first(),
  ).toBeFocused()
  await page.keyboard.press('Tab')
  await expect(page.getByRole('menu', { name: 'View' })).toBeHidden()
  await canvas.focus()
  await menuItem(page, 'Edit', /^Select All Points/)
  await expect(canvas).toBeFocused()
})

test('rejects right-button edits and cancels interrupted outline drags without history', async ({
  page,
  source,
}) => {
  await openGlyphFromGrid(page, 'A')
  const { gid } = await glyphCell(page, source.doc.doc, 'A')
  const canvas = page.getByRole('img', { name: /^Glyph / })
  const before = await outline(page, source.doc.doc, gid)
  const point = firstOnCurve(before)
  const history = () =>
    api<DocumentHistory>(page, 'doc.history', { doc: source.doc.doc })
  const initialHistory = await history()
  const tools = page.getByRole('toolbar', { name: 'Tools', exact: true })
  await tools.getByRole('button', { name: 'Pen', exact: true }).click()
  let start = await pointPosition(page, point)
  await page.mouse.move(start.x, start.y)
  await page.mouse.down({ button: 'right' })
  await page.mouse.move(start.x + 35, start.y + 20)
  await page.mouse.up({ button: 'right' })
  await tools.getByRole('button', { name: 'Select', exact: true }).click()
  expect(await outline(page, source.doc.doc, gid)).toEqual(before)
  expect(await history()).toEqual(initialHistory)

  for (const cancellation of [
    'Escape',
    'pointercancel',
    'blur',
    'right-button chord',
  ] as const) {
    start = await pointPosition(page, point)
    await canvas.focus()
    await page.mouse.move(start.x, start.y)
    await page.keyboard.down('Control')
    await page.mouse.down()
    await page.mouse.move(start.x + 35, start.y + 20)
    const cancelled = page.waitForResponse('**/api/doc.edit_cancel')
    if (cancellation === 'Escape') await page.keyboard.press('Escape')
    else if (cancellation === 'pointercancel')
      await canvas.dispatchEvent('pointercancel', {
        pointerId: 1,
        pointerType: 'mouse',
      })
    else if (cancellation === 'blur') await canvas.blur()
    else {
      // Releasing the primary button while the right one remains held must
      // cancel immediately; later right-only movement cannot continue editing.
      await page.mouse.down({ button: 'right' })
      await page.mouse.up({ button: 'left' })
      await page.mouse.move(start.x + 60, start.y + 40)
    }
    await cancelled
    await page.mouse.up({
      button: cancellation === 'right-button chord' ? 'right' : 'left',
    })
    await page.keyboard.up('Control')
    await expect.poll(() => outline(page, source.doc.doc, gid)).toEqual(before)
    expect(await history()).toEqual(initialHistory)
  }
  // A later ordinary gesture still commits once and remains independently undoable.
  start = await pointPosition(page, point)
  const end = await pointPosition(page, { x: point.x + 40, y: point.y + 20 })
  await page.mouse.move(start.x, start.y)
  await page.keyboard.down('Control')
  await page.mouse.down()
  await page.mouse.move(end.x, end.y)
  await page.mouse.up()
  await page.keyboard.up('Control')
  await expect
    .poll(async () => (await history()).cursor)
    .toBe(initialHistory.cursor + 1)
  const moved = (await outline(page, source.doc.doc, gid)).contours[
    point.contour
  ]!.points[point.point]!
  // Real browser pointer coordinates are subpixel floats after the compact
  // layout's fit transform; retain a 0.01 font-unit geometry tolerance.
  expect(moved.x).toBeCloseTo(point.x + 40, 2)
  expect(moved.y).toBeCloseTo(point.y + 20, 2)
  expect((await history()).cursor).toBe(initialHistory.cursor + 1)
  await canvas.press('Control+z')
  await expect.poll(() => outline(page, source.doc.doc, gid)).toEqual(before)
})
