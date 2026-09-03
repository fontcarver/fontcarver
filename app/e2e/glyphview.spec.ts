import { expect, test } from '@playwright/test'

/**
 * Runs against `fw-devserver --open fixtures/fonts/DejaVuSans.ttf` plus the
 * Vite dev server, like the font-view spec.
 */
test.describe('glyph view', () => {
  test('opens a glyph tab from the grid and draws its metrics', async ({
    page,
  }) => {
    await page.goto('/')
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    await expect(
      grid.getByRole('option', { name: 'A', exact: true }),
    ).toBeVisible()

    await grid.getByRole('option', { name: 'A', exact: true }).dblclick()

    await expect(page.getByRole('tab', { name: 'A', exact: true })).toBeVisible()
    await expect(
      page.getByRole('img', { name: /^Glyph A, U\+0041$/ }),
    ).toBeVisible()

    // The corner readout carries the identity and the horizontal metrics.
    const hud = page.getByLabel('Glyph readout')
    await expect(hud).toContainText('U+0041')
    await expect(hud).toContainText('1401')

    // The inspector follows the glyph without a second request.
    await expect(page.getByText('Bounds')).toBeVisible()
  })

  test('steps to the next glyph in the current order', async ({ page }) => {
    await page.goto('/')
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    await grid.getByRole('option', { name: 'A', exact: true }).dblclick()
    await expect(page.getByRole('tab', { name: 'A', exact: true })).toBeVisible()

    await page.getByRole('img', { name: /^Glyph A/ }).click()
    await page.keyboard.press('Control+]')

    await expect(page.getByRole('tab', { name: 'B', exact: true })).toBeVisible()
    await expect(page.getByRole('tab', { name: 'A', exact: true })).toHaveCount(
      0,
    )
  })

  test('shows the font info sections and searches across them', async ({
    page,
  }) => {
    await page.goto('/')
    await expect(
      page.getByRole('listbox', { name: 'Glyphs' }).getByRole('option').first(),
    ).toBeVisible()

    await page.keyboard.press('Control+i')

    await expect(page.getByRole('tab', { name: 'Font Info' })).toBeVisible()
    await expect(page.getByText('DejaVu Sans').first()).toBeVisible()

    await page.getByRole('searchbox', { name: 'Search fields' }).fill('vendor')
    await expect(page.getByText('Vendor ID')).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Font' })).toBeVisible()
  })
})
