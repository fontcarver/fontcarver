import { expect, test, type Page } from '@playwright/test'

/**
 * Drawing with the pen, end to end.
 *
 * Runs against the same self-contained servers as the other specs: a new
 * document is created through `Mod+N`, one glyph is added through the dev-only
 * kernel bridge (there is no Add Glyphs surface yet), and the triangle is drawn
 * with real pointer events on the real canvas. The assertion is what the kernel
 * stored, read back through `glyphs.outline`.
 */

/** Calls the kernel the way the page itself would. */
async function kernel<T>(
  page: Page,
  method: string,
  params: unknown,
): Promise<T> {
  return page.evaluate(
    ([name, args]) =>
      (
        globalThis as unknown as {
          __fwCall: (m: string, p: unknown) => Promise<unknown>
        }
      ).__fwCall(name as string, args),
    [method, params] as const,
  ) as Promise<T>
}

interface Summary {
  doc: number
  family_name: string
}

interface Outline {
  layer: string
  contours: { closed: boolean; points: { x: number; y: number }[] }[]
}

test.describe('drawing', () => {
  test('pen-draws a closed triangle into a new font', async ({ page }) => {
    await page.goto('/')
    await expect(
      page.getByRole('menubar', { name: 'Application menu' }),
    ).toBeVisible()

    // A new 1000 upem document, and the one glyph the pen will draw into.
    await page.keyboard.press('Control+n')
    const docs = await kernel<Summary[]>(page, 'session.list', {})
    const doc = docs[docs.length - 1]?.doc as number
    expect(doc).toBeGreaterThan(0)
    const added = await kernel<{ added: { gid: number; name: string }[] }>(
      page,
      'glyphs.add',
      { doc, recipe: 'A', master: null },
    )
    const gid = added.added[0]?.gid as number
    expect(added.added[0]?.name).toBe('A')

    // Open the glyph from the grid, which is how a user reaches the editor.
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    await expect(
      grid.getByRole('option', { name: 'A', exact: true }),
    ).toBeVisible()
    await grid.getByRole('option', { name: 'A', exact: true }).dblclick()
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
        const outline = await kernel<Outline>(page, 'glyphs.outline', {
          doc,
          gid,
          layer: null,
        })
        return outline.contours.length
      })
      .toBe(1)
    const outline = await kernel<Outline>(page, 'glyphs.outline', {
      doc,
      gid,
      layer: null,
    })
    expect(outline.contours[0]?.closed).toBe(true)
    expect(outline.contours[0]?.points).toHaveLength(3)

    // One gesture, one undo entry.
    await expect(page.getByRole('contentinfo')).toContainText('Draw path')
  })
})
