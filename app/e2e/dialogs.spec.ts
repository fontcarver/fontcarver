import { expect, test, type Page } from '@playwright/test'

import { api } from './support'

/**
 * The four dialogs that need no outline editor: New Font, Add Glyphs,
 * Preferences and Recovery. Each test works on a document it creates itself so
 * the shared kernel session stays predictable for the other specs.
 */

interface DocSummary {
  doc: number
  family_name: string
  glyph_count: number
}

interface Prefs {
  theme: string
  language: string
  recent_limit: number
  autosave_seconds: number
  grid_size: number
  snap: Record<string, boolean>
  nudge: [number, number, number]
}

/**
 * Reads the glyph count out of the status bar.
 *
 * The count is matched whole: the bar also flashes the label of the edit that
 * just happened, and "Add glyphs" would otherwise be a second match.
 */
async function glyphCount(page: Page): Promise<number> {
  const text = await page
    .getByRole('contentinfo')
    .getByText(/^[\d,]+ glyphs?$/)
    .innerText()
  return Number(text.replace(/[^\d]/g, ''))
}

async function openMenu(page: Page, menu: string, item: RegExp) {
  await page.getByRole('menuitem', { name: menu, exact: true }).click()
  await page
    .getByRole('menu', { name: menu })
    .getByRole('menuitem', { name: item })
    .click()
}

test.describe('dialogs', () => {
  test('creates a font with the Basic Latin starter set', async ({ page }) => {
    test.setTimeout(60_000)
    await page.goto('/')
    const family = `Starter ${Date.now()}`

    await openMenu(page, 'File', /^New Font/)
    const dialog = page.getByRole('dialog', { name: 'New Font' })
    await expect(dialog).toBeVisible()
    await dialog.getByLabel('Family name').fill(family)
    await dialog.getByRole('radio', { name: 'Basic Latin' }).check()
    await dialog.getByRole('button', { name: 'Create' }).click()

    await expect(page.getByRole('tab', { name: family })).toBeVisible()
    await expect
      .poll(() => glyphCount(page), { timeout: 20_000 })
      .toBeGreaterThanOrEqual(52)
  })

  test('previews a recipe and adds exactly what it previewed', async ({
    page,
  }) => {
    test.setTimeout(60_000)
    await page.goto('/')
    const family = `Recipe ${Date.now()}`
    await api<DocSummary>(page, 'session.new', {
      family,
      units_per_em: 1000,
    })
    await page.reload()
    await page.getByRole('tab', { name: family }).click()
    const before = await glyphCount(page)

    await openMenu(page, 'Glyph', /^Add Glyphs/)
    const dialog = page.getByRole('dialog', { name: 'Add Glyphs' })
    await expect(dialog).toBeVisible()
    await dialog.getByLabel('Recipe').fill('A-F uni4E00-uni4E04')

    // The preview and its counts come from a kernel dry run.
    const add = dialog.getByRole('button', { name: /^Add \d+ glyphs?$/ })
    await expect(add).toBeEnabled()
    const previewed = Number((await add.innerText()).replace(/[^\d]/g, ''))
    expect(previewed).toBe(11)
    await expect(dialog.getByText('11 new')).toBeVisible()
    await expect(
      dialog.getByRole('list', { name: 'Glyphs to add' }).getByText('uni4E00'),
    ).toBeVisible()

    await add.click()
    await expect(dialog).toBeHidden()
    await expect
      .poll(() => glyphCount(page), { timeout: 20_000 })
      .toBe(before + previewed)
  })

  test('round trips preferences through the kernel', async ({ page }) => {
    test.setTimeout(60_000)
    await page.goto('/')
    const original = await api<Prefs>(page, 'prefs.get', {})
    // Preferences are kernel-wide, so they are put back even when an assertion
    // fails: the other specs share this session.
    try {
      await runPreferencesRoundTrip(page)
    } finally {
      await api(page, 'prefs.set', { prefs: original })
    }
  })

  test('shows no recovery dialog on a clean launch', async ({ page }) => {
    await page.goto('/')
    await expect(
      page.getByRole('menubar', { name: 'Application menu' }),
    ).toBeVisible()

    expect(await api<unknown[]>(page, 'session.recover_list', {})).toEqual([])
    await expect(
      page.getByRole('dialog', { name: 'Recover Documents' }),
    ).toHaveCount(0)
  })
})

async function runPreferencesRoundTrip(page: Page) {
  await openMenu(page, 'Edit', /^Preferences/)
  const dialog = page.getByRole('dialog', { name: 'Preferences' })
  await expect(dialog).toBeVisible()

  await dialog.getByRole('tab', { name: 'Editor' }).click()
  await dialog.getByLabel('Grid size').fill('16')
  await dialog.getByLabel('Grid size').blur()
  await expect
    .poll(async () => (await api<Prefs>(page, 'prefs.get', {})).grid_size)
    .toBe(16)

  // The language switch updates the whole dialog without a reload.
  await dialog.getByRole('tab', { name: 'General' }).click()
  await dialog.getByLabel('Language').selectOption('zh-CN')
  await expect(page.getByRole('menuitem', { name: '文件' })).toBeVisible()
  await expect
    .poll(async () => (await api<Prefs>(page, 'prefs.get', {})).language)
    .toBe('zh-CN')

  await expect(page.getByRole('heading', { name: '偏好设置' })).toBeVisible()
  await page.getByRole('dialog').getByLabel('语言').selectOption('en')
  await expect(
    page.getByRole('menuitem', { name: 'File', exact: true }),
  ).toBeVisible()

  await dialog.getByRole('button', { name: 'Done' }).click()
  await expect(dialog).toBeHidden()

  // The kernel kept the change across a fresh load of the shell.
  await page.reload()
  await openMenu(page, 'Edit', /^Preferences/)
  await page
    .getByRole('dialog', { name: 'Preferences' })
    .getByRole('tab', { name: 'Editor' })
    .click()
  await expect(page.getByLabel('Grid size')).toHaveValue('16')
}
