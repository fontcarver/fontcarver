import { expect } from '@playwright/test'
import { test } from './sessionFixtures'

import { api } from './support'

/**
 * Runs against `fc-devserver --open fixtures/fonts/DejaVuSans.ttf` plus the
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
    // The field is named twice on this screen: once by the Font section's own
    // row and once by the scopes table, so the section is named here.
    await expect(
      page.getByLabel('Font', { exact: true }).getByText('Vendor ID'),
    ).toBeVisible()
    await expect(page.getByRole('heading', { name: 'Font' })).toBeVisible()
  })
})

/**
 * The editor surfaces this batch added: the Pixel tool's handover to the strike
 * editor (row 20), guide anchoring (row 30) and per-target snap distances
 * (row 31). Each one is driven the way a user reaches it and verified against
 * what the kernel stored.
 */
test.describe('editor tools, guides and snapping', () => {
  test('hands the Pixel tool over to the strike editor', async ({ page }) => {
    await page.goto('/')
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    // The grid pages 6,253 glyphs in; five seconds is not a safe budget for it
    // on a machine that is also compiling.
    await expect(
      grid.getByRole('option', { name: 'A', exact: true }),
    ).toBeVisible({ timeout: 30_000 })
    await grid.getByRole('option', { name: 'A', exact: true }).dblclick()
    const canvas = page.getByRole('img', { name: /^Glyph A/ })
    await expect(canvas).toBeVisible()

    // The palette lists it with its key, and choosing it opens the strike tab.
    const pixel = page.getByRole('button', { name: 'Pixel', exact: true })
    await expect(pixel).toBeVisible()
    await pixel.click()
    // Named exactly: a `BitmapTest.bdf` document another spec left open renders
    // its pinned tab as `Overview · BitmapTest.bdf`, which also holds "Bitmap".
    await expect(
      page.getByRole('tab', { name: 'Bitmap editor', exact: true }),
    ).toBeVisible()
    // The Edit tab is not left holding a tool that does nothing.
    await page.getByRole('tab', { name: 'A', exact: true }).click()
    await expect(pixel).toHaveAttribute('aria-pressed', 'false')
  })

  test('anchors a guide to a sidebearing and stores the rebased offset', async ({
    page,
  }) => {
    test.slow()
    await page.goto('/')
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    // The grid pages 6,253 glyphs in; five seconds is not a safe budget for it
    // on a machine that is also compiling.
    await expect(
      grid.getByRole('option', { name: 'A', exact: true }),
    ).toBeVisible({ timeout: 30_000 })
    await grid.getByRole('option', { name: 'A', exact: true }).dblclick()
    await expect(page.getByRole('img', { name: /^Glyph A/ })).toBeVisible()
    const doc = 1
    const found = await api<{ gid: number; name: string }[]>(
      page,
      'glyphs.search',
      { doc, query: 'A', limit: 20 },
    )
    const gid = found.find((cell) => cell.name === 'A')?.gid as number
    const guides = async () =>
      (
        await api<{ guides: { x: number; anchor: string }[] }>(
          page,
          'glyphs.outline',
          { doc, gid, layer: null },
        )
      ).guides

    const inspector = page.getByRole('region', { name: 'Inspector' })
    // The accordion remembers its state, so the header is only pressed when the
    // section is actually closed.
    const header = inspector.getByRole('button', { name: /^Guides/ })
    if ((await header.getAttribute('aria-expanded')) === 'false')
      await header.click()
    // The layer's own list is the first "Add vertical" in the section. Adding a
    // guide selects it, so its fields — anchoring included — are already open.
    const add = inspector.getByRole('button', { name: 'Add vertical' }).first()
    await expect(add).toBeVisible()
    await add.click()
    await expect.poll(async () => (await guides()).length).toBe(1)

    const before = (await guides())[0]?.x ?? 0
    expect(before).toBeGreaterThan(0)

    const selector = inspector.getByLabel('Anchored to').first()
    await expect(selector).toBeVisible()
    await selector.selectOption('Lsb')

    // The guide keeps its place, so the stored offset is rebased to the
    // distance from the ink's left edge: stored + left edge is where it was.
    await expect.poll(async () => (await guides())[0]?.anchor).toBe('Lsb')
    const layer = await api<{
      layer: string
      bbox: [number, number, number, number] | null
    }>(page, 'glyphs.outline', { doc, gid, layer: null })
    const stored = (await guides())[0]
    expect(stored?.x).not.toBe(before)
    expect((stored?.x ?? 0) + (layer.bbox?.[0] ?? 0)).toBe(before)

    // Leave the fixture as it was for every other spec.
    await api(page, 'doc.apply', {
      doc,
      label: 'Delete guide',
      command: {
        RemoveGuide: {
          glyph: 'A',
          layer: layer.layer,
          master: null,
          index: 0,
        },
      },
    })
    await expect.poll(async () => (await guides()).length).toBe(0)
  })

  test('gives one snap target its own reach in the preferences', async ({
    page,
  }) => {
    await page.goto('/')
    await page.getByRole('menuitem', { name: 'Edit', exact: true }).click()
    await page
      .getByRole('menu', { name: 'Edit' })
      .getByRole('menuitem', { name: /^Preferences/ })
      .click()
    const dialog = page.getByRole('dialog', { name: 'Preferences' })
    await expect(dialog).toBeVisible()
    await dialog.getByRole('tab', { name: 'Snapping' }).click()
    const nodes = dialog.getByLabel('Nodes distance (px)')
    await nodes.fill('18')
    await nodes.blur()
    await expect
      .poll(async () => {
        const prefs = await api<{ snap: { distances: { nodes: number } } }>(
          page,
          'prefs.get',
          {},
        )
        return prefs.snap.distances.nodes
      })
      .toBe(18)
    // Preferences are kernel-wide, so the default is put back.
    await nodes.fill('0')
    await nodes.blur()
    await expect
      .poll(async () => {
        const prefs = await api<{ snap: { distances: { nodes: number } } }>(
          page,
          'prefs.get',
          {},
        )
        return prefs.snap.distances.nodes
      })
      .toBe(0)
    await dialog.getByRole('button', { name: 'Done' }).click()
  })
})
