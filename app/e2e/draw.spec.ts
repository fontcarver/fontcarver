import { expect, type Page } from '@playwright/test'
import { test } from './sessionFixtures'

import { api } from './support'

/**
 * Drawing with the pen, end to end.
 *
 * The document is created through the New Font dialog and the one glyph it
 * needs through `glyphs.add`, so the spec never touches the fixture fonts the
 * other specs assert against; it closes its own document again on the way out.
 * The triangle itself is drawn with real pointer events on the real canvas and
 * the assertion is what the kernel stored.
 */

/**
 * How long a canvas click may wait for its position to be clickable.
 *
 * A point some floating panel covers can never be clicked, and without a bound
 * Playwright would retry it until the whole test times out and say nothing
 * useful. Fifteen seconds is far more than a live canvas needs and turns that
 * case into a named interception failure.
 */
const CLICK_TIMEOUT = 15_000

interface DocSummary {
  doc: number
  family_name: string
}

interface Outline {
  contours: { closed: boolean; points: { x: number; y: number }[] }[]
}

/** The document this spec created, by the family name it was given. */
async function findDoc(page: Page, family: string): Promise<number> {
  const docs = await api<DocSummary[]>(page, 'session.list', {})
  const found = docs.find((doc) => doc.family_name === family)
  if (!found) throw new Error(`no document named ${family}`)
  return found.doc
}

test.describe('drawing', () => {
  test('pen-draws a closed triangle into a new font', async ({ page }) => {
    // Creating a font, adding a glyph and drawing it is several kernel round
    // trips and a dialog; three times the configured budget, not a fixed one.
    test.slow()
    const family = `Draw ${Date.now()}`
    await page.goto('/')

    // A document of its own, with no starter glyphs to slow it down.
    await page.getByRole('menuitem', { name: 'File', exact: true }).click()
    await page
      .getByRole('menu', { name: 'File' })
      .getByRole('menuitem', { name: /^New Font/ })
      .click()
    const dialog = page.getByRole('dialog', { name: 'New Font' })
    await expect(dialog).toBeVisible()
    await dialog.getByLabel('Family name').fill(family)
    await dialog.getByRole('radio', { name: 'None', exact: true }).check()
    await dialog.getByRole('button', { name: 'Create' }).click()
    // The new document's tab has to be the active one before the grid below is
    // this font's grid; selecting it explicitly keeps the spec independent of
    // whether creating a font also focuses it.
    const tab = page.getByRole('tab', { name: family })
    await expect(tab).toBeVisible()
    await tab.click()
    await expect(tab).toHaveAttribute('aria-selected', 'true')

    const doc = await findDoc(page, family)
    const added = await api<{ added: { gid: number; name: string }[] }>(
      page,
      'glyphs.add',
      { doc, recipe: 'A', master: null, dry_run: false },
    )
    expect(added.added[0]?.name).toBe('A')
    const gid = added.added[0]?.gid as number

    try {
      // Open the glyph from the grid, which is how a user reaches the editor.
      const grid = page.getByRole('listbox', { name: 'Glyphs' })
      const option = grid.getByRole('option', { name: 'A', exact: true })
      await expect(option).toBeVisible()
      await option.dblclick()
      const canvas = page.getByRole('img', { name: /^Glyph A/ })
      await expect(canvas).toBeVisible()

      // The Pen tool, by its §5 key. The press that focuses the canvas goes to
      // its centre: the top-left corner carries the rulers and the Info box,
      // and a press that lands in a field types the key instead of using it.
      const focusBox = await canvas.boundingBox()
      if (!focusBox) throw new Error('canvas has no bounds')
      await canvas.click({
        position: { x: focusBox.width / 2, y: focusBox.height / 2 },
        timeout: CLICK_TIMEOUT,
      })
      await page.keyboard.press('p')
      await expect(
        page.getByRole('button', { name: 'Pen', exact: true }),
      ).toHaveAttribute('aria-pressed', 'true')

      // Three clicks and a click back on the first node close the path.
      const bounds = await canvas.boundingBox()
      if (!bounds) throw new Error('canvas has no bounds')
      // Draw on exposed canvas above the bottom-left Info box.
      const corners = [
        { x: bounds.width * 0.45, y: bounds.height * 0.25 },
        { x: bounds.width * 0.75, y: bounds.height * 0.25 },
        { x: bounds.width * 0.6, y: bounds.height * 0.45 },
      ]
      for (const corner of corners)
        await canvas.click({ position: corner, timeout: CLICK_TIMEOUT })
      await canvas.click({
        position: corners[0] as { x: number; y: number },
        timeout: CLICK_TIMEOUT,
      })

      // What the kernel stored, not what the canvas drew.
      await expect
        .poll(async () => {
          const outline = await api<Outline>(page, 'glyphs.outline', {
            doc,
            gid,
            layer: null,
          })
          return outline.contours.length
        })
        .toBe(1)
      const outline = await api<Outline>(page, 'glyphs.outline', {
        doc,
        gid,
        layer: null,
      })
      expect(outline.contours[0]?.closed).toBe(true)
      expect(outline.contours[0]?.points).toHaveLength(3)

      // One gesture, one undo entry.
      await expect(page.getByRole('contentinfo')).toContainText('Draw path')
    } finally {
      // Close the document created by this drawing workflow.
      await api(page, 'session.close', { doc, force: true })
    }
  })
})

/**
 * Open Corner and Reconnect Nodes, end to end (audit row 27).
 *
 * The chevron is drawn with the pen at known canvas positions, so the reflex
 * corner's position on screen is known without reading the viewport: the spec
 * clicks exactly where it clicked to create it. Both operations are driven from
 * the Path menu and every assertion is what the kernel stored.
 */
test.describe('open corner', () => {
  test('opens a reflex corner and reconnects it from the Path menu', async ({
    page,
  }) => {
    // Creating a font, adding a glyph and drawing it is several kernel round
    // trips and a dialog; three times the configured budget, not a fixed one.
    test.slow()
    const family = `Corner ${Date.now()}`
    await page.goto('/')
    await page.getByRole('menuitem', { name: 'File', exact: true }).click()
    await page
      .getByRole('menu', { name: 'File' })
      .getByRole('menuitem', { name: /^New Font/ })
      .click()
    const dialog = page.getByRole('dialog', { name: 'New Font' })
    await expect(dialog).toBeVisible()
    await dialog.getByLabel('Family name').fill(family)
    await dialog.getByRole('radio', { name: 'None', exact: true }).check()
    await dialog.getByRole('button', { name: 'Create' }).click()
    // The new document's tab has to be the active one before the grid below is
    // this font's grid; selecting it explicitly keeps the spec independent of
    // whether creating a font also focuses it.
    const tab = page.getByRole('tab', { name: family })
    await expect(tab).toBeVisible()
    await tab.click()
    await expect(tab).toHaveAttribute('aria-selected', 'true')

    const doc = await findDoc(page, family)
    const added = await api<{ added: { gid: number; name: string }[] }>(
      page,
      'glyphs.add',
      { doc, recipe: 'A', master: null, dry_run: false },
    )
    const gid = added.added[0]?.gid as number
    const points = async () => {
      const outline = await api<Outline>(page, 'glyphs.outline', {
        doc,
        gid,
        layer: null,
      })
      return outline.contours[0]?.points.length ?? 0
    }

    try {
      const grid = page.getByRole('listbox', { name: 'Glyphs' })
      const option = grid.getByRole('option', { name: 'A', exact: true })
      await expect(option).toBeVisible()
      await option.dblclick()
      const canvas = page.getByRole('img', { name: /^Glyph A/ })
      await expect(canvas).toBeVisible()
      const bounds = await canvas.boundingBox()
      if (!bounds) throw new Error('canvas has no bounds')

      // The Info box floats over the canvas's lower left, and at this window
      // size it covers a good half of a canvas that is only as tall as its
      // minimum. Every position below is therefore measured from the box's
      // right edge, so the chevron is drawn on canvas the user can actually
      // reach; a click on a covered point would otherwise wait out the test.
      const info = page.getByRole('region', { name: 'Info box' })
      const infoBounds = await info.boundingBox()
      const left = infoBounds
        ? infoBounds.x + infoBounds.width - bounds.x + 16
        : 0
      const span = bounds.width - left - 16
      if (span < 120) throw new Error('no drawable canvas beside the Info box')

      // A chevron whose middle node points back into the ink: the only kind of
      // corner a single contour can actually cross itself at.
      const at = (x: number, y: number) => ({
        x: left + span * x,
        y: bounds.height * y,
      })
      const outline = [
        at(0, 0.3),
        at(1, 0.3),
        at(0.45, 0.55),
        at(1, 0.8),
        at(0, 0.8),
      ]
      const reflex = outline[2] as { x: number; y: number }

      // A press that lands in a field would type the tool key instead of using
      // it, so the focus click goes to clear canvas too.
      await canvas.click({ position: at(0.5, 0.55), timeout: CLICK_TIMEOUT })
      await page.keyboard.press('p')
      await expect(
        page.getByRole('button', { name: 'Pen', exact: true }),
      ).toHaveAttribute('aria-pressed', 'true')
      for (const corner of outline)
        await canvas.click({ position: corner, timeout: CLICK_TIMEOUT })
      await canvas.click({
        position: outline[0] as { x: number; y: number },
        timeout: CLICK_TIMEOUT,
      })
      await expect.poll(points).toBe(5)

      // Select the corner with the Select tool and open it.
      await page.keyboard.press('v')
      await canvas.click({ position: reflex, timeout: CLICK_TIMEOUT })
      // By role and exact name: "Collapse info box" also contains an X.
      await expect(
        info.getByRole('textbox', { name: 'X', exact: true }),
      ).toBeVisible()

      const menubar = page.getByRole('menubar')
      await menubar.getByRole('menuitem', { name: 'Path', exact: true }).click()
      await page.getByRole('menuitem', { name: /^Open Corner/ }).click()
      await expect.poll(points).toBe(6)
      await expect(page.getByRole('contentinfo')).toContainText('Open corner')

      // Marquee the mouth: both of its nodes sit within a few pixels of where
      // the corner was, and the neighbouring nodes are a third of the canvas
      // away. The drag starts inside the ink, which is not hit-testable.
      await page.mouse.move(reflex.x + bounds.x - 30, reflex.y + bounds.y - 22)
      await page.mouse.down()
      await page.mouse.move(reflex.x + bounds.x + 18, reflex.y + bounds.y + 22)
      await page.mouse.up()
      await expect(info).toContainText('2 of')

      await menubar.getByRole('menuitem', { name: 'Path', exact: true }).click()
      await page.getByRole('menuitem', { name: /^Reconnect Nodes/ }).click()
      await expect.poll(points).toBe(5)
      await expect(page.getByRole('contentinfo')).toContainText(
        'Reconnect nodes',
      )

      // Two gestures, two undo entries.
      await page.keyboard.press('Control+z')
      await expect.poll(points).toBe(6)
      await page.keyboard.press('Control+z')
      await expect.poll(points).toBe(5)
    } finally {
      await api(page, 'session.close', { doc, force: true })
    }
  })
})
