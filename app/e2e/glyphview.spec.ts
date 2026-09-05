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

    await expect(
      page.getByRole('tab', { name: 'A', exact: true }),
    ).toBeVisible()
    await expect(
      page.getByRole('img', { name: /^Glyph A, U\+0041$/ }),
    ).toBeVisible()

    // The Info box carries the identity and the horizontal metrics.
    const info = page.getByRole('region', { name: 'Info box' })
    await expect(info).toContainText('U+0041')
    await expect(info.getByLabel('advance')).toHaveValue('1401')

    // The inspector follows the glyph without a second request.
    await expect(page.getByText('Bounds')).toBeVisible()
  })

  test('steps to the next glyph in the current order', async ({ page }) => {
    await page.goto('/')
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    // Wait for the cell before double-clicking it, as the spec above does: the
    // grid's focus bookkeeping runs on the pointer-down, and a cell whose page
    // is still in flight is a placeholder that carries no glyph to open.
    await expect(
      grid.getByRole('option', { name: 'A', exact: true }),
    ).toBeVisible()
    await grid.getByRole('option', { name: 'A', exact: true }).dblclick()
    await expect(
      page.getByRole('tab', { name: 'A', exact: true }),
    ).toBeVisible()

    await page.getByRole('img', { name: /^Glyph A/ }).click()
    await page.keyboard.press('Control+]')

    await expect(
      page.getByRole('tab', { name: 'B', exact: true }),
    ).toBeVisible()
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
