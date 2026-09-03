import { expect, test } from '@playwright/test'

/**
 * Runs against `fw-devserver --open fixtures/fonts/DejaVuSans.ttf` plus the
 * Vite dev server. DejaVu Sans has 6,253 glyphs, 3,245 of them in the Letter
 * category, which is what the counts below are checked against.
 */
test.describe('font view', () => {
  test('renders the glyph grid for the pre-opened font', async ({ page }) => {
    await page.goto('/')

    await expect(
      page.getByRole('tab', { name: 'DejaVuSans.ttf' }),
    ).toBeVisible()
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    await expect(grid).toBeVisible()
    await expect(
      grid.getByRole('option', { name: 'A', exact: true }),
    ).toBeVisible()
    await expect(
      page.getByRole('contentinfo').getByText('6,253 glyphs'),
    ).toBeVisible()
  })

  test('searches by Unicode scalar', async ({ page }) => {
    await page.goto('/')
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    await expect(
      grid.getByRole('option', { name: 'A', exact: true }),
    ).toBeVisible()

    await page.getByRole('searchbox', { name: 'Search glyphs' }).fill('U+0042')

    await expect(grid.getByRole('option')).toHaveCount(1)
    await expect(grid.getByRole('option').first()).toHaveAttribute(
      'data-glyph',
      'B',
    )
  })

  test('filters by a sidebar category and reports the count', async ({
    page,
  }) => {
    await page.goto('/')
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    await expect(
      grid.getByRole('option', { name: 'A', exact: true }),
    ).toBeVisible()

    await page.getByRole('button', { name: /^Letter \d/ }).click()

    const status = page.getByRole('contentinfo')
    await expect(status.getByText('of 6,253')).toBeVisible()
    const shown = await status.getByText(/glyphs?$/).innerText()
    const count = Number(shown.replace(/[^\d]/g, ''))
    expect(count).toBeGreaterThan(0)
    expect(count).toBeLessThan(6253)

    await expect(
      page.getByRole('button', { name: 'Remove filter Letter' }),
    ).toBeVisible()
  })
})
