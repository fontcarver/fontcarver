import { existsSync, readFileSync, statSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, isAbsolute, join, resolve } from 'node:path'

import { expect } from '@playwright/test'
import { test } from './sessionFixtures'

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

/**
 * These cases need no CJK corpus: they run against the committed DejaVu
 * document the default configuration opens.
 *
 * The vertical metrics are given to the layer through the inspector, which is
 * row 19's own entry point and is also what makes the canvas's vertical lines
 * exist, so one document covers both rows.
 */
test.describe('vertical metrics, the canvas handles and the radical library', () => {
  /** Font units per CSS pixel and the canvas y of one font y. */
  async function mapping(page: import('@playwright/test').Page) {
    const box = (await page
      .getByRole('img', { name: /^Glyph A, U\+0041$/ })
      .boundingBox())!
    const read = async (x: number, y: number) => {
      await page.mouse.move(x, y)
      const text = await page.getByRole('contentinfo').innerText()
      const match = /(-?[\d,]+), (-?[\d,]+)/.exec(text)
      if (!match) throw new Error(`no pointer readout in: ${text}`)
      return {
        x: Number(match[1]!.replace(/,/g, '')),
        y: Number(match[2]!.replace(/,/g, '')),
      }
    }
    const top = { x: box.x + box.width / 2, y: box.y + box.height * 0.25 }
    const low = { x: top.x, y: box.y + box.height * 0.75 }
    const a = await read(top.x, top.y)
    const b = await read(low.x, low.y)
    const units = (a.y - b.y) / (low.y - top.y)
    return {
      box,
      units,
      screenY: (font: number) => top.y + (a.y - font) / units,
      // The metric line spans the canvas, so the gesture is driven near the
      // right edge, clear of the outline and of the advance vertical.
      centreX: box.x + box.width * 0.9,
    }
  }

  async function openGlyphA(page: import('@playwright/test').Page) {
    await page.goto('/')
    const grid = page.getByRole('listbox', { name: 'Glyphs' })
    const cell = grid.getByRole('option', { name: 'A', exact: true })
    await expect(cell).toBeVisible()
    await cell.dblclick()
    await expect(
      page.getByRole('img', { name: /^Glyph A, U\+0041$/ }),
    ).toBeVisible()
  }

  test('edits TSB and BSB, then drags the origin only in vertical mode', async ({
    page,
  }) => {
    await openGlyphA(page)

    // Row 19: the four fields, and TSB moving the origin while the advance
    // stays put.
    const advance = page.getByLabel('vert. advance')
    const origin = page.getByLabel('vert. origin')
    const tsb = page.getByLabel('TSB')
    const bsb = page.getByLabel('BSB')
    const staleTsb = await tsb.inputValue()
    await advance.fill('2400')
    await advance.press('Enter')
    await origin.fill('1200')
    await origin.press('Enter')
    // TSB and BSB are derived from the kernel's resolved ink, so they are the
    // fields that say the model caught up: the origin box shows what was typed
    // the moment it is typed.
    await expect(origin).toHaveValue('1200')
    await expect(tsb).not.toHaveValue(staleTsb)
    const firstTsb = Number(await tsb.inputValue())
    const firstBsb = Number(await bsb.inputValue())

    await tsb.fill(String(firstTsb + 100))
    await tsb.press('Enter')
    // Moving TSB by 100 moves the origin by 100 and leaves the advance alone.
    await expect(origin).toHaveValue('1300')
    await expect(advance).toHaveValue('2400')
    await expect(bsb).toHaveValue(String(firstBsb - 100))

    // Row 111: out of vertical mode the origin line is drawn but is not a
    // handle, so a press on it must leave the metric alone.
    const { units, screenY, centreX } = await mapping(page)
    const line = screenY(1300)
    await page.mouse.move(centreX, line)
    await page.mouse.down()
    await page.mouse.move(centreX, line - 20)
    await page.mouse.up()
    // Give a write that should not happen time to arrive before denying it.
    await page.waitForTimeout(500)
    await expect(origin).toHaveValue('1300')

    await page.getByRole('menuitem', { name: 'View', exact: true }).click()
    await page
      .getByRole('menu', { name: 'View' })
      .getByRole('menuitem', { name: 'Vertical Writing Mode' })
      .click()

    // In vertical mode the same gesture moves the origin by the distance
    // dragged, and leaves the advance exactly as it was.
    await page.mouse.move(centreX, line)
    await page.mouse.down()
    await page.mouse.move(centreX, line - 10)
    await page.mouse.move(centreX, line - 20)
    await page.mouse.up()
    await expect(origin).not.toHaveValue('1300')
    const moved = Number(await origin.inputValue())
    expect(moved).toBeGreaterThan(1300)
    expect(Math.abs(moved - (1300 + 20 * units))).toBeLessThanOrEqual(2)
    await expect(advance).toHaveValue('2400')
    // One history entry for the whole drag.
    await page.getByRole('menuitem', { name: 'Edit', exact: true }).click()
    await expect(
      page
        .getByRole('menu', { name: 'Edit' })
        .getByRole('menuitem', { name: /^Undo Set vertical metrics/ }),
    ).toBeVisible()
    await page.keyboard.press('Escape')
  })

  test('lists the pinned radicals even in a font with no ideographs', async ({
    page,
  }) => {
    await openGlyphA(page)
    await page.getByRole('button', { name: 'CJK construction' }).click()
    await page.getByRole('button', { name: 'Radical library' }).click()

    // DejaVu has none of the radical forms, which the library says rather than
    // pretending the index is empty.
    const onlyPresent = page.getByLabel('Only forms this font has')
    await expect(onlyPresent).toBeChecked()
    await expect(
      page.getByText(/No matching radicals in this font/),
    ).toBeVisible()

    // The whole index comes from the shipped Unihan data: 214 block characters
    // plus the ideographs that write them.
    await onlyPresent.uncheck()
    await expect(page.getByText(/of 525$/)).toBeVisible()
    await expect(
      page.getByRole('button', {
        name: /Insert Radical 1 · 1 stroke · U\+2F00/,
      }),
    ).toBeVisible()
    await expect(page.getByText(/Unicode 17\.0\.0 Unihan/)).toBeVisible()

    // Searching by radical number narrows to that radical's written forms.
    await page.getByLabel('Search radicals and components').fill('85')
    for (const scalar of ['U\\+6C34', 'U\\+6C35', 'U\\+6C3A']) {
      await expect(
        page.getByRole('button', {
          name: new RegExp(`Insert Radical 85 · \\d+ strokes? · ${scalar}`),
        }),
      ).toBeVisible()
    }

    // The tab strip is one tab stop with arrow-key navigation.
    const radicals = page.getByRole('tab', { name: 'Radicals' })
    await radicals.focus()
    await page.keyboard.press('ArrowRight')
    await expect(
      page.getByRole('tab', { name: 'Font components' }),
    ).toHaveAttribute('aria-selected', 'true')
    await expect(
      page.getByText(/This font uses no components yet/),
    ).toBeVisible()
  })
})
