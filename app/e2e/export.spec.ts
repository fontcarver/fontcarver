import { existsSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { expect, type Page } from '@playwright/test'
import { test } from './sessionFixtures'

import { api, fixture, inspect } from './support'

/**
 * The P1 exit criterion through the dialog: open a font, edit one outline,
 * export a TTF and prove the file on disk is a font with the same glyph
 * repertoire. Task 12 adds the variant that performs the edit with the pointer
 * once the outline editor is merged.
 */

interface DocSummary {
  doc: number
  path: string | null
  glyph_count: number
}

interface Cell {
  gid: number
  name: string
}

interface Outline {
  layer: string
  contours: { points: { x: number; y: number }[] }[]
}

const FONT = fixture('DejaVuSansMin.woff2')

async function openFixture(page: Page): Promise<DocSummary> {
  await page.goto('/')
  const summary = await api<DocSummary>(page, 'session.open', {
    path: FONT,
    face_index: null,
  })
  // The shell lists the open documents once, at start-up.
  await page.reload()
  return summary
}

test.describe('export', () => {
  let directory = ''

  test.beforeAll(() => {
    directory = mkdtempSync(join(tmpdir(), 'fc-e2e-export-'))
  })

  test.afterAll(() => {
    rmSync(directory, { recursive: true, force: true })
  })

  test('edits a glyph and exports a TTF the CLI can read back', async ({
    page,
  }) => {
    test.setTimeout(180_000)
    const summary = await openFixture(page)

    // One MovePoints on `A`, through the same API the editor will use.
    const cells = await api<Cell[]>(page, 'glyphs.search', {
      doc: summary.doc,
      query: 'A',
      limit: 16,
    })
    const glyph = cells.find((cell) => cell.name === 'A')
    expect(glyph, 'the fixture has a glyph named A').toBeTruthy()
    const outline = await api<Outline>(page, 'glyphs.outline', {
      doc: summary.doc,
      gid: glyph?.gid,
      layer: null,
    })
    const before = outline.contours[0]?.points[0]
    expect(before, 'A has an outline to edit').toBeTruthy()

    await api(page, 'doc.apply', {
      doc: summary.doc,
      command: {
        MovePoints: {
          glyph: 'A',
          layer: outline.layer,
          points: [{ contour: 0, point: 0 }],
          delta: [10, 0],
          handles: 'PointsOnly',
        },
      },
      label: 'Move 1 point',
    })
    const moved = await api<Outline>(page, 'glyphs.outline', {
      doc: summary.doc,
      gid: glyph?.gid,
      layer: null,
    })
    expect(moved.contours[0]?.points[0]?.x).toBeCloseTo(
      (before?.x ?? 0) + 10,
      6,
    )

    // Export through the dialog.
    await page.getByRole('tab', { name: 'DejaVuSansMin.woff2' }).click()
    await page.getByRole('menuitem', { name: 'File', exact: true }).click()
    await page
      .getByRole('menu', { name: 'File' })
      .getByRole('menuitem', { name: /^Export/ })
      .click()

    const dialog = page.getByRole('dialog', { name: 'Export' })
    await expect(dialog).toBeVisible()
    await dialog.getByRole('tab', { name: 'TTF', exact: true }).click()

    const out = join(directory, 'DejaVuSansMin.ttf')
    await dialog.getByLabel('Destination path').fill(out)
    await dialog.getByRole('button', { name: 'Export', exact: true }).click()

    await expect(dialog.getByText(out)).toBeVisible({ timeout: 120_000 })
    await expect(
      dialog.getByRole('button', { name: 'Export again' }),
    ).toBeVisible()

    expect(existsSync(out), `${out} was written`).toBe(true)
    const report = inspect(out)
    expect(report.summary.glyph_count).toBe(summary.glyph_count)
  })
})

/**
 * Row 58: an export profile is project data.
 *
 * The profile is saved through the dialog, read back out of the document by the
 * kernel, and applied again after a reload — which is what makes the same
 * record available to `fontcarver build --profile`.
 *
 * The session fixture supplies this test's own DejaVu document.
 */
test.describe('export profiles', () => {
  test('saves a profile into the document and applies it after a reload', async ({
    page,
  }) => {
    test.setTimeout(180_000)
    await page.goto('/')
    const docs = await api<DocSummary[]>(page, 'session.list', {})
    const startup = docs.find((doc) => doc.path?.endsWith('DejaVuSans.ttf'))
    expect(startup, 'the session fixture opened its font').toBeTruthy()
    const doc = startup!.doc
    await page.reload()

    const openDialog = async () => {
      await page.getByRole('tab', { name: 'DejaVuSans.ttf' }).click()
      await page.getByRole('menuitem', { name: 'File', exact: true }).click()
      await page
        .getByRole('menu', { name: 'File' })
        .getByRole('menuitem', { name: /^Export/ })
        .click()
      const dialog = page.getByRole('dialog', { name: 'Export' })
      await expect(dialog).toBeVisible()
      return dialog
    }

    const dialog = await openDialog()
    await dialog.getByRole('tab', { name: 'WOFF2', exact: true }).click()
    await dialog.getByLabel('Profile name').fill('Web')
    await dialog.getByLabel('Output subfolder').fill('web')
    await dialog.getByRole('button', { name: 'Save profile' }).click()
    await expect(dialog.getByRole('option', { name: 'Web' })).toBeAttached()

    // The kernel, not the browser, is holding it.
    const stored = await api<{
      profiles: {
        name: string
        subfolder: string
        options: { format: string }
      }[]
    }>(page, 'export.profiles_get', { doc })
    expect(stored.profiles).toHaveLength(1)
    expect(stored.profiles[0]).toMatchObject({
      name: 'Web',
      subfolder: 'web',
      options: { format: 'Woff2' },
    })

    // A reload keeps it, and applying it restores the format it names.
    await page.reload()
    const reopened = await openDialog()
    await reopened.getByRole('tab', { name: 'TTF', exact: true }).click()
    await reopened.getByLabel('Saved profile').selectOption({ label: 'Web' })
    await reopened.getByRole('button', { name: 'Apply profile' }).click()
    await expect(
      reopened.getByRole('tab', { name: 'WOFF2', exact: true }),
    ).toHaveAttribute('aria-selected', 'true')
    await expect(reopened.getByLabel('Output subfolder')).toHaveValue('web')
  })
})
