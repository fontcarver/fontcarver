import { expect } from '@playwright/test'

import type { Prefs } from '../src/api/generated/Prefs'
import { enterText, test } from './p2aSupport'
import { api } from './support'

test('steps both sidebearings by ×100 or exactly one without consuming glyph navigation', async ({
  page,
  source,
}) => {
  const prefs = await api<Prefs>(page, 'prefs.get', {})
  await api(page, 'prefs.set', { prefs: { ...prefs, nudge: [2.5, 25, 250] } })
  await page.reload()
  const canvas = await enterText(page, 'AV')
  // Typing enters proofing mode; selecting the first shaped glyph enables
  // sidebearing editing, exactly as in the user's canvas workflow.
  const at = await canvas.evaluate((element) => {
    const globals = globalThis as unknown as {
      __fwTextRun?: { glyphs: { x: number; y: number; xAdvance: number }[] }
      __fwEditor?: {
        viewportForTest: {
          toScreen: (point: { x: number; y: number }) => {
            x: number
            y: number
          }
        }
      }
    }
    const glyph = globals.__fwTextRun?.glyphs[0]
    const editor = globals.__fwEditor
    if (!glyph || !editor) throw new Error('first shaped glyph is unavailable')
    const point = editor.viewportForTest.toScreen({
      x: glyph.x + glyph.xAdvance / 2,
      y: glyph.y + 300,
    })
    const rect = (
      element as unknown as {
        getBoundingClientRect: () => { left: number; top: number }
      }
    ).getBoundingClientRect()
    return { x: rect.left + point.x, y: rect.top + point.y }
  })
  await page.mouse.click(at.x, at.y)
  await expect(page.locator('[data-text-mode]')).toHaveAttribute(
    'data-text-mode',
    'edit',
  )
  await expect(page.getByRole('textbox', { name: 'LSB key' })).toBeVisible()
  await page
    .getByRole('combobox', { name: 'Spacing side' })
    .selectOption('both')
  const metrics = async (glyph = 'A') => {
    const result = await api<{
      rows: { width: number; lsb: number; rsb: number }[]
    }>(page, 'metrics.spacing', {
      doc: source.doc.doc,
      master: source.doc.masters[0]!.id,
      glyphs: [glyph],
      beam: null,
    })
    const row = result.rows[0]!
    return { width: row.width, lsb: row.lsb, rsb: row.rsb }
  }
  const before = await metrics()
  const afterLarge = {
    width: before.width + 500,
    lsb: before.lsb + 250,
    rsb: before.rsb + 250,
  }
  const afterUnit = {
    width: afterLarge.width - 2,
    lsb: afterLarge.lsb - 1,
    rsb: afterLarge.rsb - 1,
  }
  const ready = async (expected: typeof before) => {
    await expect(page.locator('[data-text-mode]')).toHaveAttribute(
      'data-text-shaping',
      'false',
    )
    await expect(
      page
        .getByRole('region', { name: 'Inspector', exact: true })
        .getByRole('textbox', { name: 'LSB', exact: true }),
    ).toHaveValue(String(expected.lsb))
  }
  const step = async (keys: string, expected: typeof before) => {
    await Promise.all([
      page.waitForResponse('**/api/doc.edit_end').then((response) => {
        expect(response.ok()).toBe(true)
      }),
      canvas.press(keys).then(async () => {
        await expect.poll(metrics).toEqual(expected)
      }),
    ])
    // The closed kernel transaction triggers outline and shaping reloads. Wait
    // for the visible editor to catch up before starting the next gesture.
    await ready(expected)
  }
  await step('Alt+]', afterLarge)
  await step('Alt+Shift+[', afterUnit)

  // Each settled gesture contributes exactly one undo entry.
  for (const [keys, expected] of [
    ['Control+z', afterLarge],
    ['Control+z', before],
    ['Control+Shift+z', afterLarge],
    ['Control+Shift+z', afterUnit],
  ] as const) {
    await canvas.press(keys)
    await expect.poll(metrics).toEqual(expected)
    await ready(expected)
  }

  await page
    .getByRole('combobox', { name: 'Kerning mode' })
    .selectOption('locked')
  await expect(
    page.getByRole('combobox', { name: 'Spacing side' }),
  ).toBeDisabled()
  await canvas.press('Alt+]')
  await canvas.press('Alt+Shift+[')
  // The locked controls must neither apply nor schedule a delayed metrics run.
  await page.waitForTimeout(600)
  expect(await metrics()).toEqual(afterUnit)

  await page.getByRole('combobox', { name: 'Kerning mode' }).selectOption('on')
  const glyphs = ['.notdef', 'A', 'V', 'a', 'v']
  const beforeNavigation = await Promise.all(glyphs.map(metrics))
  await canvas.press('Control+]')
  // Document navigation follows the font's order, not the proof string's AV.
  await expect(canvas).toHaveAttribute('aria-label', /^Glyph (?!A,).+/)
  expect(await Promise.all(glyphs.map(metrics))).toEqual(beforeNavigation)
})
