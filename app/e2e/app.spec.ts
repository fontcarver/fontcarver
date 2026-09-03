import { expect, test } from '@playwright/test'

test('shows the workspace shell', async ({ page }) => {
  await page.goto('/')

  await expect(
    page.getByRole('heading', { name: 'FontWorkshop' }),
  ).toBeVisible()
})
