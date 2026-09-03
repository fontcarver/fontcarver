import { existsSync } from 'node:fs'

import { expect, test } from '@playwright/test'

const SOURCE_HAN =
  '/home/chen/githubprojects/pixelfonts/SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf'

test.describe('CJK font view', () => {
  test.skip(!existsSync(SOURCE_HAN), `missing external fixture: ${SOURCE_HAN}`)

  test('opens Source Han Sans and renders cells at the end of the grid', async ({
    page,
  }) => {
    test.setTimeout(30_000)
    await page.goto('/')

    await page.getByRole('menuitem', { name: 'File', exact: true }).click()
    await page
      .getByRole('menu', { name: 'File' })
      .getByRole('menuitem', { name: /Open Font/ })
      .click()
    const prompt = page.getByRole('dialog', { name: 'Open Font' })
    await expect(prompt).toBeVisible()
    await prompt.getByRole('textbox', { name: 'File path' }).fill(SOURCE_HAN)
    await prompt.getByRole('button', { name: 'Open', exact: true }).click()

    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    await expect(grid).toBeVisible()
    await expect(
      page.getByRole('contentinfo').getByText('65,535 glyphs'),
    ).toBeVisible({ timeout: 20_000 })

    await grid.evaluate((element) => {
      const scroller = element as unknown as {
        scrollTop: number
        readonly scrollHeight: number
      }
      scroller.scrollTop = scroller.scrollHeight
    })

    const lastCell = grid.locator('[role="option"][aria-posinset="65535"]')
    await expect(lastCell).toBeVisible({ timeout: 10_000 })
    await expect(lastCell).toHaveAttribute('data-index', '65534')
    await expect(lastCell).toHaveAttribute('data-glyph', /.+/)
  })
})
