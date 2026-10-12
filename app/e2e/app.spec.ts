import { expect, type Page } from '@playwright/test'
import { test } from './sessionFixtures'

test('shows the workspace shell', async ({ page }) => {
  await page.goto('/')

  // The empty state is only rendered while no document is open, and the dev
  // session fixture opens one, so assert the always-present chrome instead.
  await expect(
    page.getByRole('menubar', { name: 'Application menu' }),
  ).toBeVisible()
  await expect(page.getByRole('contentinfo')).toBeVisible()
})

/** Opens one DejaVu glyph in an Edit tab, leaving it selected. */
async function openGlyph(page: Page, name: string): Promise<void> {
  const grid = page.getByRole('listbox', { name: 'Glyphs' })
  const cell = grid.getByRole('option', { name, exact: true })
  // The grid is virtualised and its cells arrive with their thumbnails, so the
  // cell is waited for rather than assumed to be in the first paint.
  await expect(cell).toBeAttached({ timeout: 20_000 })
  await cell.dblclick()
  await expect(
    page.getByRole('img', { name: new RegExp(`^Glyph ${name}`) }),
  ).toBeVisible()
}

test.describe('shell windows and tabs', () => {
  test('pins the Overview and reaches every tab by its ordinal', async ({
    page,
  }) => {
    await page.goto('/')
    await openGlyph(page, 'K')
    await page.keyboard.press('Control+i') // Font Info

    // The Font Info view has a tablist of its own, so the shell strip is named.
    const tabs = page
      .getByRole('tablist', { name: 'Open tabs' })
      .getByRole('tab')
    await expect(tabs).toHaveCount(3)
    const overview = tabs.nth(0)
    await expect(overview).toHaveAttribute('data-pinned', 'true')
    await expect(overview).toHaveAttribute('aria-keyshortcuts', 'Control+Alt+1')
    // The pinned tab has no close control; the document is closed from File.
    await expect(
      page.getByRole('button', { name: /^Close Overview/ }),
    ).toHaveCount(0)

    await page.keyboard.press('Control+Alt+1')
    await expect(overview).toHaveAttribute('aria-selected', 'true')
    await page.keyboard.press('Control+Alt+3')
    await expect(tabs.nth(2)).toHaveAttribute('aria-selected', 'true')
    await page.keyboard.press('Control+Alt+2')
    await expect(tabs.nth(1)).toHaveAttribute('aria-selected', 'true')

    // Conventional cycling wraps in both directions.
    await page.keyboard.press('Control+Tab')
    await expect(tabs.nth(2)).toHaveAttribute('aria-selected', 'true')
    await page.keyboard.press('Control+Shift+Tab')
    await expect(tabs.nth(1)).toHaveAttribute('aria-selected', 'true')

    // Close Tab refuses the pinned Overview and says why.
    await page.keyboard.press('Control+Alt+1')
    const menubar = page.getByRole('menubar')
    await menubar.getByRole('menuitem', { name: 'Window', exact: true }).click()
    const closeTab = page.getByRole('menuitem', { name: /^Close Tab/ })
    await expect(closeTab).toBeDisabled()
    await expect(closeTab).toHaveAttribute('aria-description', /pinned/i)
    await page.keyboard.press('Escape')
  })

  test('tears the Inspector into its own window and takes it back', async ({
    page,
  }) => {
    await page.goto('/')
    await expect(page.getByRole('region', { name: 'Inspector' })).toBeVisible()

    const popupPromise = page.waitForEvent('popup')
    await page.getByRole('button', { name: 'Detach inspector' }).click()
    const popup = await popupPromise
    await popup.waitForLoadState('domcontentloaded')

    // The dock gives up its space while the window holds the view.
    await expect(page.getByRole('region', { name: 'Inspector' })).toHaveCount(0)
    await expect(popup.getByRole('region', { name: 'Inspector' })).toBeVisible()

    // Returning closes the window the click happened in, so the click is not
    // awaited for a response that can never arrive; the outcome is the dock.
    await Promise.all([
      popup.waitForEvent('close'),
      popup
        .getByRole('button', { name: 'Return to document window' })
        .click({ noWaitAfter: true })
        .catch(() => undefined),
    ])
    await expect(page.getByRole('region', { name: 'Inspector' })).toBeVisible()
  })
})

test.describe('a detached panel of an Edit tab', () => {
  test('shows live glyph content and follows the glyph', async ({ page }) => {
    await page.goto('/')
    await openGlyph(page, 'A')

    const popupPromise = page.waitForEvent('popup')
    await page.getByRole('button', { name: 'Detach inspector' }).click()
    const popup = await popupPromise
    await popup.waitForLoadState('domcontentloaded')

    // The window has no canvas of its own, so this is the case that used to
    // wait for a glyph nobody was going to publish.
    const inspector = popup.getByRole('region', { name: 'Inspector' })
    await expect(inspector).toBeVisible()
    await expect(inspector.getByLabel('LSB', { exact: true })).toBeVisible({
      timeout: 20_000,
    })
    await expect(popup.getByText('Loading glyph…')).toHaveCount(0)
    const lsb = inspector.getByLabel('LSB', { exact: true })
    const forA = await lsb.inputValue()
    expect(Number(forA)).toBeGreaterThan(0)

    // The owning window steps to another glyph; the popup follows it.
    await page
      .getByRole('region', { name: 'Sidebar' })
      .getByRole('listbox', { name: 'Glyph list' })
      .locator('[data-glyph="B"]')
      .click()
    await expect(page.getByRole('img', { name: /^Glyph B/ })).toBeVisible()
    await expect
      .poll(async () => lsb.inputValue(), { timeout: 20_000 })
      .not.toBe(forA)

    await Promise.all([
      popup.waitForEvent('close'),
      popup
        .getByRole('button', { name: 'Return to document window' })
        .click({ noWaitAfter: true })
        .catch(() => undefined),
    ])
    await expect(page.getByRole('region', { name: 'Inspector' })).toBeVisible()

    // Closing the Edit tab returns this document to its Overview.
    await page.keyboard.press('Control+Shift+W')
    await expect(page.getByRole('img', { name: /^Glyph / })).toHaveCount(0)
  })
})

test.describe('command palette', () => {
  test('searches commands, filters and glyph names in scoped sections', async ({
    page,
  }) => {
    await page.goto('/')
    await expect(page.getByRole('listbox', { name: 'Glyphs' })).toBeVisible()

    await page.keyboard.press('Control+k')
    const dialog = page.getByRole('dialog', { name: 'Commands' })
    await expect(dialog).toBeVisible()
    await dialog.getByRole('combobox', { name: 'Commands' }).fill('aacute')

    const listbox = dialog.getByRole('listbox')
    const glyphs = listbox.getByRole('group', { name: 'Glyphs' })
    await expect(glyphs).toBeVisible()
    // Every populated category is its own labelled section.
    await expect(listbox.getByRole('group')).not.toHaveCount(0)

    await glyphs
      .getByRole('option', { name: /^aacute/ })
      .first()
      .click()
    // The Edit tab names itself after the text it shapes, so the canvas is
    // what says which glyph was opened.
    await expect(page.getByRole('img', { name: /^Glyph aacute/ })).toBeVisible()
    await expect(dialog).toHaveCount(0)
  })
})

test.describe('document history', () => {
  test('lists one entry per gesture with its scope and moves back through it', async ({
    page,
  }) => {
    await page.goto('/')
    // Use E for the history gesture and its visible scope label.
    await openGlyph(page, 'E')

    const inspector = page.getByRole('region', { name: 'Inspector' })
    const lsb = inspector.getByLabel('LSB', { exact: true })
    await expect(lsb).toBeVisible()
    const original = await lsb.inputValue()
    await lsb.fill('210')
    await lsb.press('Enter')
    await expect(lsb).toHaveValue('210')

    await inspector.getByRole('button', { name: /^History/ }).click()
    const list = inspector.getByRole('list', { name: 'History entries' })
    await expect(list).toBeVisible()
    const entries = list.getByRole('button')
    // One typed field commit is one entry, scoped to the glyph and its layer.
    await expect(entries.first()).toHaveAttribute('aria-current', 'true')
    await expect(entries.first()).toContainText('Glyphs: E')

    await entries.last().click()
    await expect(lsb).toHaveValue(original)
    await expect(entries.last()).toBeDisabled()
    // The undone entry is still listed, so it can be reapplied from the list.
    await expect(entries.first()).toContainText('undone')
    await entries.first().click()
    await expect(lsb).toHaveValue('210')

    // Verify that returning to the initial entry restores the metrics.
    await entries.last().click()
    await expect(lsb).toHaveValue(original)
  })
})

test.describe('preview strip', () => {
  test('resizes with its divider and collapses from the keyboard', async ({
    page,
  }) => {
    await page.goto('/')
    await openGlyph(page, 'H')

    const strip = page.getByRole('region', { name: 'Preview' })
    const divider = page.getByRole('separator', { name: 'Resize preview' })
    await expect(divider).toBeVisible()
    const before = (await strip.boundingBox())?.height ?? 0

    const box = await divider.boundingBox()
    if (!box) throw new Error('the preview divider has no box')
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2)
    await page.mouse.down()
    await page.mouse.move(box.x + box.width / 2, box.y - 80, { steps: 8 })
    await page.mouse.up()
    const after = (await strip.boundingBox())?.height ?? 0
    expect(after).toBeGreaterThan(before + 40)

    // The keyboard reaches the same handle.
    await divider.focus()
    await divider.press('ArrowDown')
    expect(Number(await divider.getAttribute('aria-valuenow'))).toBeLessThan(
      after,
    )

    await page.keyboard.press('Control+Alt+p')
    await expect(
      page.getByRole('button', { name: 'Expand preview' }),
    ).toBeVisible()
    await expect(divider).toHaveCount(0)
    await page.keyboard.press('Control+Alt+p')
    await expect(divider).toBeVisible()
  })
})

test.describe('reopening a saved file', () => {
  test('gives each document exactly one pinned Overview', async ({ page }) => {
    // Two cold imports and their tab transitions are a multi-document workflow.
    test.slow()
    await page.goto('/')
    const strip = page.getByRole('tablist', { name: 'Open tabs' })
    await expect(strip.getByRole('tab').first()).toBeVisible()
    const before = await strip.getByRole('tab').count()

    const open = async () => {
      await page
        .getByRole('menubar')
        .getByRole('menuitem', { name: 'File', exact: true })
        .click()
      await page.getByRole('menuitem', { name: /^Open Font/ }).click()
      const dialog = page.getByRole('dialog', { name: 'Open Font' })
      await expect(dialog).toBeVisible()
      await dialog
        .getByRole('textbox', { name: 'File path' })
        .fill('fixtures/fonts/NotoSans-Regular.ttf')
      await dialog.getByRole('button', { name: 'Open', exact: true }).click()
      await expect(dialog).toBeHidden()
    }

    await open()
    const overview = strip.getByRole('tab', {
      name: /· NotoSans-Regular\.ttf$/,
    })
    await expect(overview).toHaveCount(1)
    await expect(overview).toHaveAttribute('aria-selected', 'true')
    const first = await strip
      .getByRole('tab')
      .and(page.locator('[data-pinned="true"]'))
      .count()

    // Opening the same file again is a second *document* — comparing a file
    // with the copy you have edited is an accepted workflow — and each document
    // contributes exactly one pinned Overview, never two for one document.
    await open()
    await expect(overview).toHaveCount(2)
    await expect(overview.last()).toHaveAttribute('aria-selected', 'true')
    await expect(
      strip.getByRole('tab').and(page.locator('[data-pinned="true"]')),
    ).toHaveCount(first + 1)
    await expect(strip.getByRole('tab')).toHaveCount(before + 2)

    // Closing the documents removes exactly their own tabs.
    await page.keyboard.press('Control+w')
    await expect(overview).toHaveCount(1)
    await overview.click()
    await page.keyboard.press('Control+w')
    await expect(overview).toHaveCount(0)
  })
})

test.describe('a second document', () => {
  test('opens with its own Overview selected, beside the first document', async ({
    page,
  }) => {
    await page.goto('/')
    const strip = page.getByRole('tablist', { name: 'Open tabs' })
    await expect(strip.getByRole('tab').first()).toBeVisible()
    // Record the first document's tabs before opening the second one.
    const before = await strip.getByRole('tab').count()

    // Leave the first document on a Glyph tab: opening another document still
    // has to show the document that was opened.
    await openGlyph(page, 'A')

    await page
      .getByRole('menubar')
      .getByRole('menuitem', { name: 'File', exact: true })
      .click()
    await page.getByRole('menuitem', { name: /^Open Font/ }).click()
    const dialog = page.getByRole('dialog', { name: 'Open Font' })
    await expect(dialog).toBeVisible()
    await dialog
      .getByRole('textbox', { name: 'File path' })
      .fill('fixtures/fonts/NotoSans-Regular.ttf')
    await dialog.getByRole('button', { name: 'Open', exact: true }).click()
    await expect(dialog).toBeHidden()

    const opened = strip.getByRole('tab', { name: /· NotoSans-Regular\.ttf$/ })
    await expect(opened).toHaveAttribute('aria-selected', 'true')
    await expect(opened).toHaveAttribute('data-pinned', 'true')
    // The first document keeps its own run of tabs, Overview first.
    await expect(strip.getByRole('tab')).toHaveCount(before + 2)
    await expect(strip.getByRole('tab').nth(0)).toHaveAttribute(
      'data-pinned',
      'true',
    )
    await expect(strip.getByRole('tab').nth(1)).not.toHaveAttribute(
      'data-pinned',
      'true',
    )

    // File ▸ Close Font closes the active second document.
    await page.keyboard.press('Control+w')
    await expect(opened).toHaveCount(0)
  })
})
