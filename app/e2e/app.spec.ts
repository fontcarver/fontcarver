import { expect, test } from '@playwright/test'

test('shows the workspace shell', async ({ page }) => {
  await page.goto('/')

  // The empty state is only rendered while no document is open, and the dev
  // server opens one at startup, so assert the always-present chrome instead.
  await expect(
    page.getByRole('menubar', { name: 'Application menu' }),
  ).toBeVisible()
  await expect(page.getByRole('contentinfo')).toBeVisible()
})
