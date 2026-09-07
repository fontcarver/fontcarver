import { expect, test, type Page } from '@playwright/test'

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
    await expect(page.getByRole('tab', { name: family })).toBeVisible()

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

      // The Pen tool, by its §5 key.
      await canvas.click({ position: { x: 10, y: 10 } })
      await page.keyboard.press('p')
      await expect(
        page.getByRole('button', { name: 'Pen', exact: true }),
      ).toHaveAttribute('aria-pressed', 'true')

      // Three clicks and a click back on the first node close the path.
      const corners = [
        { x: 140, y: 320 },
        { x: 300, y: 320 },
        { x: 220, y: 180 },
      ]
      for (const corner of corners) await canvas.click({ position: corner })
      await canvas.click({ position: corners[0] as { x: number; y: number } })

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
      // The kernel session is shared with every other spec.
      await api(page, 'session.close', { doc, force: true })
    }
  })
})
