import { existsSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

import { expect, test } from '@playwright/test'

const SOURCE_HAN =
  '/home/chen/githubprojects/pixelfonts/SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf'

/**
 * The Playwright config prefers this binary when it exists, and only it opens
 * a 65,535-glyph OTF inside the transport's 30 s request timeout: a debug
 * kernel needs 18-75 s for the same font, so the spec would fail on speed
 * rather than on behaviour.
 */
const RELEASE_KERNEL = fileURLToPath(
  new URL('../../target/release/fw-devserver', import.meta.url),
)

test.describe('CJK font view', () => {
  test.skip(!existsSync(SOURCE_HAN), `missing external fixture: ${SOURCE_HAN}`)
  test.skip(
    !existsSync(RELEASE_KERNEL),
    'needs a release fw-devserver: cargo build --release -p fw-devserver',
  )

  test('opens Source Han Sans and renders cells at the end of the grid', async ({
    page,
  }) => {
    // A debug `fw-devserver` needs about 75 s to parse a 65,535-glyph OTF; a
    // release build does it in under 3 s (docs/p0-acceptance.md). The suite
    // runs against the debug binary, so the budget is the debug one.
    test.setTimeout(240_000)
    await page.goto('/')

    await page.getByRole('menuitem', { name: 'File', exact: true }).click()
    await page
      .getByRole('menu', { name: 'File' })
      .getByRole('menuitem', { name: /Open Font/ })
      .click()
    const prompt = page.getByRole('dialog', { name: 'Open Font' })
    await expect(prompt).toBeVisible()
    await prompt.getByRole('textbox', { name: 'File path' }).fill(SOURCE_HAN)
    await prompt.getByRole('button', { name: 'Open', exact: true }).click()

    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    await expect(grid).toBeVisible()
    await expect(
      page.getByRole('contentinfo').getByText('65,535 glyphs'),
    ).toBeVisible({ timeout: 180_000 })

    await grid.evaluate((element) => {
      const scroller = element as unknown as {
        scrollTop: number
        readonly scrollHeight: number
      }
      scroller.scrollTop = scroller.scrollHeight
    })

    // The last page of a 65,535-glyph list carries 300 flattened outlines,
    // which a debug kernel takes tens of seconds to produce.
    const lastCell = grid.locator('[role="option"][aria-posinset="65535"]')
    await expect(lastCell).toBeVisible({ timeout: 60_000 })
    await expect(lastCell).toHaveAttribute('data-index', '65534')
    await expect(lastCell).toHaveAttribute('data-glyph', /.+/)
  })
})
