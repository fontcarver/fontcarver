import { existsSync, readFileSync, statSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, isAbsolute, join, resolve } from 'node:path'

import { expect, test } from '@playwright/test'

function primaryRepositoryRoot(): string {
  const worktreeRoot = fileURLToPath(new URL('../..', import.meta.url))
  const dotGit = join(worktreeRoot, '.git')
  if (!existsSync(dotGit) || statSync(dotGit).isDirectory()) return worktreeRoot
  const pointer = readFileSync(dotGit, 'utf8').trim()
  const match = /^gitdir:\s*(.+)$/u.exec(pointer)
  if (!match?.[1]) return worktreeRoot
  const gitDir = isAbsolute(match[1])
    ? match[1]
    : resolve(worktreeRoot, match[1])
  return resolve(gitDir, '../../..')
}

const SOURCE_HAN =
  process.env.FC_SOURCE_HAN_SANS ||
  resolve(
    dirname(primaryRepositoryRoot()),
    'SourceHanSanOTF/SimplifiedChinese/SourceHanSansSC-Regular.otf',
  )

/**
 * The Playwright config prefers this binary when it exists, and only it opens
 * a 65,535-glyph OTF inside the transport's 30 s request timeout: a debug
 * kernel needs 18-75 s for the same font, so the spec would fail on speed
 * rather than on behaviour.
 */
const RELEASE_KERNEL = fileURLToPath(
  new URL('../../target/release/fc-devserver', import.meta.url),
)

test.describe('CJK font view', () => {
  test.skip(!existsSync(SOURCE_HAN), `missing external fixture: ${SOURCE_HAN}`)
  test.skip(
    !existsSync(RELEASE_KERNEL),
    'needs a release fc-devserver: cargo build --release -p fc-devserver',
  )

  test('opens Source Han Sans and renders cells at the end of the grid', async ({
    page,
  }) => {
    // The skip above guarantees a release kernel, which opens this font in
    // under 3 s (docs/p0-acceptance.md); the budgets are sized for that, so a
    // real regression fails fast instead of after minutes of waiting.
    test.setTimeout(60_000)
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
    ).toBeVisible({ timeout: 20_000 })

    await grid.evaluate((element) => {
      const scroller = element as unknown as {
        scrollTop: number
        readonly scrollHeight: number
      }
      scroller.scrollTop = scroller.scrollHeight
    })

    // The last page of a 65,535-glyph list carries 300 flattened outlines.
    const lastCell = grid.locator('[role="option"][aria-posinset="65535"]')
    await expect(lastCell).toBeVisible({ timeout: 10_000 })
    await expect(lastCell).toHaveAttribute('data-index', '65534')
    await expect(lastCell).toHaveAttribute('data-glyph', /.+/)
  })
})
