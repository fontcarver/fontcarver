import { expect, type Page } from '@playwright/test'
import { test } from './sessionFixtures'

/** Opens the app after the isolated fixture has finished importing its font. */
async function openFont(page: Page): Promise<void> {
  await page.goto('/')
  const tab = page.getByRole('tab', { name: 'DejaVuSans.ttf' })
  await expect(tab).toBeVisible({ timeout: 20_000 })
}

/**
 * Each test opens a fresh copy of DejaVu Sans, which has 6,253 glyphs,
 * 3,245 of them in the Letter category. Counts are checked against that font.
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
    // Match the count whole, separately from the status bar's edit messages.
    const shown = await status.getByText(/^[\d,]+ glyphs?$/).innerText()
    const count = Number(shown.replace(/[^\d]/g, ''))
    expect(count).toBeGreaterThan(0)
    expect(count).toBeLessThan(6253)

    await expect(
      page.getByRole('button', { name: 'Remove filter Letter' }),
    ).toBeVisible()
  })

  test('reports the selection in the status bar', async ({ page }) => {
    await page.goto('/')
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    const status = page.getByRole('contentinfo')

    await expect(status.getByText('0 selected')).toBeVisible()

    await grid.getByRole('option', { name: 'A', exact: true }).click()
    await expect(status.getByText('1 selected')).toBeVisible()

    await grid.getByRole('option', { name: 'D', exact: true }).click({
      modifiers: ['Shift'],
    })
    await expect(status.getByText('4 selected')).toBeVisible()
  })
})

test.describe('font view list mode', () => {
  test('sorts by every column, chooses columns and keeps the choice', async ({
    page,
  }) => {
    await openFont(page)
    await expect(
      page
        .getByRole('listbox', { name: 'Glyphs' })
        .getByRole('option', { name: 'A', exact: true }),
    ).toBeVisible()

    await page.getByRole('button', { name: 'List view' }).click()
    const rows = page.getByRole('rowgroup', { name: 'Glyph rows' })
    const firstGlyph = async () => {
      await expect(rows.getByRole('row').first()).toBeVisible()
      return rows.getByRole('row').first().getAttribute('data-glyph')
    }
    const inFontOrder = await firstGlyph()

    // Every column heading sorts, including the three the kernel gained.
    const heads = page.getByRole('grid', { name: 'Glyph list' })
    for (const column of [
      'Name',
      'Unicode',
      'Width',
      'Category',
      'Script',
      'Status',
    ]) {
      await page.getByRole('button', { name: column, exact: true }).click()
      await expect(
        heads.getByRole('columnheader', { name: new RegExp(`^${column}`) }),
      ).toHaveAttribute('aria-sort', 'ascending')
    }
    // The kernel really reorders: narrowest first once Width is chosen.
    await page.getByRole('button', { name: 'Width', exact: true }).click()
    await expect(
      rows.getByRole('row').first().getByRole('gridcell').nth(2),
    ).toHaveText('0')
    expect(inFontOrder).toBeTruthy()

    await page.getByRole('button', { name: 'Choose columns' }).click()
    await page
      .getByRole('group', { name: 'Choose columns' })
      .getByRole('checkbox', { name: 'Subcategory' })
      .click()
    await page.keyboard.press('Escape')
    await expect(
      heads.getByRole('columnheader', { name: /^Subcategory/ }),
    ).toBeVisible()

    // The column set survives a reload: it is a workspace preference.
    await page.reload()
    await expect(
      page
        .getByRole('grid', { name: 'Glyph list' })
        .getByRole('columnheader', { name: /^Subcategory/ }),
    ).toBeVisible()
    await page.getByRole('button', { name: 'Choose columns' }).click()
    await page
      .getByRole('group', { name: 'Choose columns' })
      .getByRole('checkbox', { name: 'Subcategory' })
      .click()
    await page.keyboard.press('Escape')
    await page.getByRole('button', { name: 'Grid view' }).click()
  })
})

test.describe('font view search and filters', () => {
  test('reports an invalid regular expression under the field', async ({
    page,
  }) => {
    await openFont(page)
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    await expect(
      grid.getByRole('option', { name: 'A', exact: true }),
    ).toBeVisible()

    await page.getByRole('button', { name: 'Use regular expression' }).click()
    const search = page.getByRole('searchbox', { name: 'Search glyphs' })
    await search.fill('[')

    const alert = page.getByRole('alert').filter({ hasText: 'regex' })
    await expect(alert).toBeVisible()
    await expect(search).toHaveAttribute('aria-invalid', 'true')

    // Completing the pattern clears the report and filters the grid. The
    // pattern is case-insensitive until Match case is on.
    await search.fill('^A$')
    await expect(alert).toBeHidden()
    await expect(grid.getByRole('option')).toHaveCount(2)
    const matchCase = page.getByRole('button', {
      name: 'Match case',
      exact: true,
    })
    await matchCase.click()
    await expect(grid.getByRole('option')).toHaveCount(1)
    await expect(grid.getByRole('option').first()).toHaveAttribute(
      'data-glyph',
      'A',
    )
    await matchCase.click()
    await page
      .getByRole('button', { name: 'Use regular expression', exact: true })
      .click()
    await search.fill('')
  })

  test('badges a Unicode block with its coverage and filters by a badge', async ({
    page,
  }) => {
    await openFont(page)
    await expect(
      page
        .getByRole('listbox', { name: 'Glyphs' })
        .getByRole('option', { name: 'A', exact: true }),
    ).toBeVisible()

    // Basic Latin is fully assigned, so its denominator is its 128 characters.
    const block = page.getByRole('button', { name: /^Basic Latin/ })
    await expect(block).toContainText('/ 128')
    await expect(block).toHaveAttribute('title', /characters encoded/)

    await page.getByRole('button', { name: 'Not exported' }).click()
    await expect(
      page.getByRole('button', { name: 'Not exported' }),
    ).toHaveAttribute('aria-pressed', 'true')
    await expect(page.getByRole('contentinfo')).toContainText('of 6,253')
  })
})

test.describe('font view editing workflows', () => {
  test('labels a whole selection in one undo step', async ({ page }) => {
    await openFont(page)
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    const cell = grid.getByRole('option', { name: 'A', exact: true })
    await expect(cell).toBeVisible()
    await cell.click()
    // The command acts on the selection, so prove which cell is in it.
    await expect(cell).toHaveAttribute('aria-selected', 'true')

    await page
      .getByRole('menubar')
      .getByRole('menuitem', { name: 'Glyph', exact: true })
      .click()
    await page
      .getByRole('menu', { name: 'Glyph' })
      .getByRole('menuitem', { name: 'Red', exact: true })
      .click()

    // The sidebar's colour section follows a document event, which can lag
    // behind the edit on a loaded machine.
    const colours = page.getByRole('button', { name: /^Red/ })
    await expect(colours).toBeVisible({ timeout: 20_000 })
    await page.keyboard.press('Control+z')
    await expect(colours).toBeHidden({ timeout: 20_000 })
  })

  test('generates a composite from the glyph database', async ({ page }) => {
    // The smallest window the shell targets: a dialog taller than this must
    // scroll rather than push its confirm button off-screen.
    await page.setViewportSize({ width: 1280, height: 720 })
    await openFont(page)
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    await expect(
      grid.getByRole('option', { name: 'A', exact: true }),
    ).toBeVisible()

    await page.getByRole('searchbox', { name: 'Search glyphs' }).fill('f_f')
    await expect(grid.getByRole('option')).toHaveCount(0)
    await page.getByRole('searchbox', { name: 'Search glyphs' }).fill('')

    await page
      .getByRole('menubar')
      .getByRole('menuitem', { name: 'Glyph', exact: true })
      .click()
    await page
      .getByRole('menu', { name: 'Glyph' })
      .getByRole('menuitem', { name: 'Add Glyphs…' })
      .click()
    const dialog = page.getByRole('dialog', { name: 'Add Glyphs' })
    await expect(dialog).toBeVisible()
    await dialog.getByLabel('Recipe').fill('f_f')
    await expect(
      dialog.getByLabel('Build composites from the glyph database'),
    ).toBeChecked()
    await expect(dialog.getByText('1 composite')).toBeVisible()
    await dialog.getByRole('button', { name: /^Add 1 glyph/ }).click()
    await expect(dialog).toBeHidden()

    await page.getByRole('searchbox', { name: 'Search glyphs' }).fill('f_f')
    const created = grid.getByRole('option', { name: /^f_f/ })
    await expect(created).toHaveCount(1, { timeout: 20_000 })
    // A composite: the cell marks its components, and it is not empty.
    await expect(created.first()).toHaveAttribute('data-glyph', 'f_f')

    // Undo must remove the generated composite, too.
    await page.keyboard.press('Control+z')
    await expect(created).toHaveCount(0, { timeout: 20_000 })
  })

  test('renames a selection with a regular expression', async ({ page }) => {
    await openFont(page)
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    await page.getByRole('searchbox', { name: 'Search glyphs' }).fill('aacute')
    const cell = grid.getByRole('option', { name: /^aacute/ })
    await expect(cell).toHaveCount(1)
    // Click re-resolves and scrolls the locator if filtering remounts the grid.
    await cell.click()
    await expect(cell).toHaveAttribute('aria-selected', 'true')

    await page
      .getByRole('menubar')
      .getByRole('menuitem', { name: 'Glyph', exact: true })
      .click()
    await page
      .getByRole('menu', { name: 'Glyph' })
      .getByRole('menuitem', { name: /^Batch Rename/ })
      .click()
    const dialog = page.getByRole('dialog', { name: 'Batch Rename Glyphs…' })
    await dialog.getByLabel('Regular expression').check()
    await dialog.getByLabel('Find').fill('^(a)acute$')
    await dialog.getByLabel('Replace with').fill('$1.acute')
    await expect(
      dialog.getByRole('table', { name: 'Glyph rename preview' }),
    ).toContainText('a.acute')
    await dialog.getByRole('button', { name: /^Rename 1 glyph/ }).click()
    await expect(dialog).toBeHidden()

    await page.getByRole('searchbox', { name: 'Search glyphs' }).fill('a.acute')
    await expect(grid.getByRole('option', { name: /^a\.acute/ })).toHaveCount(
      1,
      { timeout: 20_000 },
    )

    await page.keyboard.press('Control+z')
    await expect(grid.getByRole('option', { name: /^a\.acute/ })).toHaveCount(
      0,
      { timeout: 20_000 },
    )
  })
})
