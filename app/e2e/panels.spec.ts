import { expect, type Page } from '@playwright/test'
import { test } from './sessionFixtures'

/**
 * The Edit tab's inspector panels and the Path menu, against
 * an isolated copy of `fixtures/fonts/DejaVuSans.ttf`.
 *
 * Every step is one undoable command, so the spec ends by walking the history
 * back and checking that the layer really is where it started: that is the
 * property the panels are for — a numeric route to a change that produces the
 * same undo history as the dragged one.
 */

/** Opens DejaVu `A` in an Edit tab. */
async function openA(page: Page): Promise<void> {
  await page.goto('/')
  const grid = page.getByRole('listbox', { name: 'Glyphs' })
  const cell = grid.getByRole('option', { name: 'A', exact: true })
  // The grid is virtualised and its cells arrive with their thumbnails.
  await expect(cell).toBeAttached({ timeout: 20_000 })
  await cell.dblclick()
  await expect(page.getByRole('tab', { name: 'A', exact: true })).toBeVisible()
  await expect(page.getByRole('img', { name: /^Glyph A/ })).toBeVisible()
}

/**
 * Runs one item of the in-app menu bar. A menu item's accessible name carries
 * its shortcut hint after the title, so the item is matched by prefix.
 */
async function runMenu(page: Page, menu: string, item: string): Promise<void> {
  const menubar = page.getByRole('menubar')
  await menubar.getByRole('menuitem', { name: menu, exact: true }).click()
  await page
    .getByRole('menuitem', {
      name: new RegExp(`^${item.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`),
    })
    .click()
}

/** The label of the entry the Edit menu would undo, or `null`. */
async function undoLabel(page: Page): Promise<string | null> {
  const menubar = page.getByRole('menubar')
  await menubar.getByRole('menuitem', { name: 'Edit', exact: true }).click()
  const item = page.getByRole('menuitem', { name: /^Undo/ })
  const text = await item.textContent()
  await page.keyboard.press('Escape')
  return text?.replace(/\s+/g, ' ').trim() ?? null
}

/** Focus the editor for its shortcut and wait for the kernel history change. */
async function undo(page: Page): Promise<void> {
  const [response] = await Promise.all([
    page.waitForResponse('**/api/doc.undo'),
    page.getByRole('img', { name: /^Glyph A/ }).press('Control+z'),
  ])
  expect((await response.json()).error).toBeUndefined()
}

test.describe('editor panels', () => {
  test('edits metrics, geometry and anchors, then undoes all of it', async ({
    page,
  }) => {
    await openA(page)

    // Every test gets a clean document and must restore its initial history.
    const undoBefore = await undoLabel(page)

    const inspector = page.getByRole('region', { name: 'Inspector' })
    const lsb = inspector.getByLabel('LSB', { exact: true })
    await expect(lsb).toBeVisible()
    const originalLsb = await lsb.inputValue()
    expect(Number(originalLsb)).toBeGreaterThan(0)
    const originalWidth = await inspector
      .getByLabel('advance', { exact: true })
      .inputValue()

    // 1. The Metrics panel writes one SetMetrics.
    await lsb.fill('200')
    await lsb.press('Enter')
    await expect(lsb).toHaveValue('200')

    // The Info box reads the same layer, so it agrees without a reload.
    await expect(
      page.getByRole('region', { name: 'Info box' }).getByLabel('LSB'),
    ).toHaveValue('200')
    const metricsUndo = await undoLabel(page)

    // 2. The Path menu runs a whole-layer operation. The advance is the same
    // afterwards, so the Edit menu is what says the command actually ran.
    await runMenu(page, 'Path', 'Remove Overlap')
    expect(await undoLabel(page)).toContain('Remove overlap')

    // 3. The Anchors panel adds an anchor with the first free name.
    await inspector.getByRole('button', { name: 'Add anchor' }).click()
    await expect(
      inspector.getByRole('button', { name: 'Delete anchor top' }),
    ).toBeVisible()

    // Three edits, three undos: the anchor goes, the outline operation goes,
    // and the layer is back at the metrics the font shipped.
    await undo(page)
    await expect(
      inspector.getByRole('button', { name: 'Delete anchor top' }),
    ).toHaveCount(0)
    await undo(page)
    // Wait for the UI to show the remaining metrics entry as well.
    await expect.poll(() => undoLabel(page)).toBe(metricsUndo)
    await undo(page)
    await expect(lsb).toHaveValue(originalLsb)
    await expect(inspector.getByLabel('advance', { exact: true })).toHaveValue(
      originalWidth,
    )
    // Nothing of this test left on the stack: the undo item reads exactly what
    // it read before the three edits.
    expect(await undoLabel(page)).toBe(undoBefore)
  })

  test('switches glyph from the sidebar and layer from the layer list', async ({
    page,
  }) => {
    await openA(page)

    const sidebar = page.getByRole('region', { name: 'Sidebar' })
    // The compact glyph list is the Font view's grid under its current filter.
    const list = sidebar.getByRole('listbox', { name: 'Glyph list' })
    await expect(list.locator('[data-gid]').first()).toBeVisible()
    await list.locator('[data-glyph="B"]').click()

    await expect(
      page.getByRole('tab', { name: 'B', exact: true }),
    ).toBeVisible()
    await expect(page.getByRole('tab', { name: 'A', exact: true })).toHaveCount(
      0,
    )

    // A background layer added from the Layer menu shows up in both lists.
    await runMenu(page, 'Layer', 'Add Background')
    await expect(
      sidebar.getByRole('button', { name: 'Background' }),
    ).toBeVisible()
    await sidebar.getByRole('button', { name: 'Background' }).click()
    await expect(
      sidebar.getByRole('button', { name: 'Background' }),
    ).toHaveAttribute('aria-current', 'true')

    await page.keyboard.press('Control+z')
    await expect(
      sidebar.getByRole('button', { name: 'Background' }),
    ).toHaveCount(0)
  })
})

test.describe('saved workspaces', () => {
  test('saves a named arrangement and restores it on demand', async ({
    page,
  }) => {
    await openA(page)
    const inspector = page.getByRole('region', { name: 'Inspector' })
    await expect(inspector).toBeVisible()

    // An arrangement worth naming: no Inspector, no preview strip.
    await page.keyboard.press('Control+2')
    await expect(inspector).toHaveCount(0)
    await page.keyboard.press('Control+3')
    await expect(page.getByRole('region', { name: 'Preview' })).toHaveCount(0)

    await runMenu(page, 'Window', 'Workspaces')
    const dialog = page.getByRole('dialog', { name: 'Saved workspaces' })
    await expect(dialog).toBeVisible()
    // The dialog states where the arrangement is kept.
    await expect(dialog).toContainText(/Saved (in this document|on this)/)
    await dialog.getByLabel('Workspace name').fill('Canvas only')
    await dialog.getByRole('button', { name: 'Save workspace' }).click()
    await expect(dialog).toContainText('Workspace saved.')
    // The dialog owns the keyboard while it is open, which is why the panels
    // are changed back with it closed.
    await dialog.getByRole('button', { name: 'Close' }).click()
    await expect(dialog).toHaveCount(0)

    // Undo the arrangement by hand, then let the saved one put it back.
    await page.keyboard.press('Control+2')
    await page.keyboard.press('Control+3')
    await expect(inspector).toBeVisible()
    await expect(page.getByRole('region', { name: 'Preview' })).toBeVisible()

    await runMenu(page, 'Window', 'Workspaces')
    await expect(dialog).toBeVisible()
    await dialog.getByRole('button', { name: 'Apply Canvas only' }).click()
    await expect(dialog).toContainText('Applied Canvas only.')
    await expect(inspector).toHaveCount(0)
    await expect(page.getByRole('region', { name: 'Preview' })).toHaveCount(0)

    await dialog.getByRole('button', { name: 'Delete Canvas only' }).click()
    await expect(dialog).toContainText('Deleted Canvas only.')
    await dialog.getByRole('button', { name: 'Close' }).click()
    await expect(dialog).toHaveCount(0)
  })
})
