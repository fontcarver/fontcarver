import { execFileSync } from 'node:child_process'
import { writeFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { expect, type Page } from '@playwright/test'
import { test } from './sessionFixtures'

import {
  menuItem,
  openGlyphFromGrid,
  openPathThroughDialog,
  type DocSummary,
} from './acceptanceSupport'
import { api, fixture, REPO_ROOT } from './support'

interface HintState {
  generation: number
  contour_count: number
  flex_paths: string[]
  hints: {
    flex_hints: { contour: number; point: number; depth: number }[] | undefined
    dot_sections: number[] | undefined
  } | null
  master: { id: string }
  layer: string | null
}

/**
 * A project whose one outline is the cupped serif of Type 1 figure 8e, already
 * carrying the flex hint that covers its two curve segments.
 *
 * The geometry is built here rather than committed so the spec states exactly
 * what it is hinting; the command line only converts the committed fixture.
 */
function flexProject(): string {
  const path = test.info().outputPath('flex-demo.fcf')
  const binary = resolve(REPO_ROOT, 'target', 'debug', 'fontcarver')
  execFileSync(binary, ['convert', fixture('SourceSans3Min.otf'), path], {
    cwd: REPO_ROOT,
  })
  const document = JSON.parse(
    execFileSync('cat', [path], { encoding: 'utf8' }),
  ) as {
    font: { glyphs: Record<string, { layers: Record<string, unknown>[] }> }
  }
  const point = (x: number, y: number, kind: string) => ({ x, y, kind })
  const layer = document.font.glyphs.o?.layers[0] as Record<string, unknown>
  layer.contours = [
    {
      closed: true,
      points: [
        point(100, 690, 'Line'),
        point(115, 690, 'OffCurve'),
        point(125, 700, 'OffCurve'),
        point(150, 700, 'Curve'),
        point(175, 700, 'OffCurve'),
        point(185, 690, 'OffCurve'),
        point(200, 690, 'Curve'),
        point(200, 0, 'Line'),
        point(100, 0, 'Line'),
      ],
    },
  ]
  layer.hints = {
    h_stems: [],
    v_stems: [],
    hint_masks: [],
    flex_hints: [{ contour: 0, point: 0, depth: 50 }],
  }
  writeFileSync(path, JSON.stringify(document))
  return path
}

async function hintState(
  page: Page,
  doc: number,
  master: string,
): Promise<HintState> {
  return api<HintState>(page, 'hints.state', {
    doc,
    master,
    glyph: 'o',
    layer: null,
  })
}

async function openHintEditor(page: Page): Promise<void> {
  const open = page.getByRole('button', { name: 'PostScript hints' })
  if (!(await open.isVisible())) {
    await page.getByRole('button', { name: 'HINTING' }).click()
  }
  await open.click()
  await expect(
    page.getByRole('heading', { name: 'PostScript hints' }),
  ).toBeVisible()
}

test('flex hints and dot sections are edited, applied and refused when invalid', async ({
  page,
}) => {
  await page.goto('/')
  const opened: DocSummary = await openPathThroughDialog(page, flexProject())
  await openGlyphFromGrid(page, 'o')
  const master = (
    await api<{ masters: { id: string }[] }>(page, 'doc.summary', {
      doc: opened.doc,
    })
  ).masters[0]!.id
  await openHintEditor(page)

  // The imported flex hint is editable data, and its resolved curve pair is
  // drawn over the outline preview.
  const before = await hintState(page, opened.doc, master)
  expect(before.hints?.flex_hints).toEqual([
    { contour: 0, point: 0, depth: 50 },
  ])
  expect(before.flex_paths[0]).toContain('C')
  await expect(page.getByRole('textbox', { name: 'Flex depth' })).toHaveValue(
    '50',
  )
  // The preview highlights the resolved curve pair; the fields below it edit it.
  await page
    .getByRole('img', { name: /numbered horizontal and vertical stem hints/ })
    .screenshot({ path: test.info().outputPath('flex-preview.png') })
  await page
    .getByRole('textbox', { name: 'Flex depth' })
    .scrollIntoViewIfNeeded()
  await page.screenshot({ path: test.info().outputPath('flex-editor.png') })

  // Changing the depth and adding a dot section applies as one undo step.
  await page.getByRole('textbox', { name: 'Flex depth' }).fill('60')
  await page.getByRole('textbox', { name: 'Flex depth' }).press('Tab')
  await page.getByRole('button', { name: 'Add dot section' }).click()
  await page.getByRole('button', { name: 'Apply changes' }).click()
  await expect(
    page.getByRole('heading', { name: 'PostScript hints' }),
  ).toBeHidden()
  const applied = await hintState(page, opened.doc, master)
  expect(applied.hints?.flex_hints).toEqual([
    { contour: 0, point: 0, depth: 60 },
  ])
  expect(applied.hints?.dot_sections).toEqual([0])
  expect(applied.generation).toBeGreaterThan(before.generation)

  // A flex hint bound to a point that does not start two curves is refused by
  // the kernel; the dialog keeps the draft and the document is untouched.
  await openHintEditor(page)
  await page.getByRole('textbox', { name: 'Point index' }).fill('7')
  await page.getByRole('textbox', { name: 'Point index' }).press('Tab')
  await page.getByRole('button', { name: 'Apply changes' }).click()
  await expect(page.getByRole('alert')).toBeVisible()
  await expect(page.getByRole('textbox', { name: 'Point index' })).toHaveValue(
    '7',
  )
  await page.screenshot({ path: test.info().outputPath('flex-refused.png') })
  const unchanged = await hintState(page, opened.doc, master)
  expect(unchanged.hints?.flex_hints).toEqual(applied.hints?.flex_hints)
  expect(unchanged.generation).toBe(applied.generation)
  await page.getByRole('button', { name: 'Cancel' }).click()

  // The applied edit survives one undo and redo as a single step.
  await menuItem(page, 'Edit', /^Undo/)
  await expect
    .poll(
      async () => (await hintState(page, opened.doc, master)).hints?.flex_hints,
    )
    .toEqual([{ contour: 0, point: 0, depth: 50 }])
  await menuItem(page, 'Edit', /^Redo/)
  await expect
    .poll(
      async () =>
        (await hintState(page, opened.doc, master)).hints?.dot_sections,
    )
    .toEqual([0])
})

test('the export dialog discloses whether the external TrueType hinter is installed', async ({
  page,
}) => {
  await page.goto('/')
  const status = await api<{ installed: boolean; version: string }>(
    page,
    'hints.tt_autohinter',
    {},
  )
  await menuItem(page, 'File', /^Export/)
  const dialog = page.getByRole('dialog', { name: 'Export' })
  await expect(dialog).toBeVisible()
  await dialog.getByRole('tab', { name: 'TTF', exact: true }).click()
  await dialog.getByRole('checkbox', { name: 'Automatic hinting' }).check()
  const section = dialog.getByRole('group', {
    name: 'External TrueType hinter',
  })
  await expect(section).toBeVisible()
  await expect(section.getByRole('status')).toHaveText(
    status.installed
      ? new RegExp(`ttfautohint found`)
      : /ttfautohint is not installed on PATH/,
  )
  // The documented options reach the kernel, which validates them.
  await expect(
    section.getByRole('spinbutton', { name: 'Hinting range minimum (ppem)' }),
  ).toBeVisible()
  await expect(
    section.getByRole('textbox', { name: 'Default script' }),
  ).toBeVisible()
  await page.screenshot({ path: test.info().outputPath('autohint-status.png') })
  await dialog.getByRole('button', { name: 'Close' }).click()
})
