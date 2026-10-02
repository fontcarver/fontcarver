import { existsSync } from 'node:fs'
import { join } from 'node:path'

import { expect, type Page } from '@playwright/test'

import type { DocSummary } from '../src/api/generated/DocSummary'
import type { GlyphKerning } from '../src/api/generated/GlyphKerning'
import {
  exportThroughDialog,
  menuItem,
  openGlyphFromGrid,
} from './acceptanceSupport'
import {
  enterText,
  expectAdvance,
  expectPreviewAdvance,
  holdNextKerning,
  openSource,
  resolution,
  saveAs,
  selectFirstGap,
  sourceFixture,
  test,
} from './p2aSupport'
import { api, fixture } from './support'

async function seedGroups(page: Page, doc: DocSummary) {
  for (const [side, name, members] of [
    ['side1', 'public.kern1.A', ['A', 'a']],
    ['side2', 'public.kern2.V', ['V', 'v']],
  ]) {
    await api(page, 'doc.apply', {
      doc: doc.doc,
      label: 'Prepare group fixture',
      command: { SetKerningGroup: { side, name, members } },
    })
  }
}

async function choosePair(page: Page) {
  const panel = page.getByTestId('text-kerning-panel')
  await expect(panel).toBeVisible()
  await page.getByTestId('text-kerning-first').fill('A')
  await page.getByTestId('text-kerning-first').press('Enter')
  await page.getByTestId('text-kerning-second').fill('V')
  await page.getByTestId('text-kerning-second').press('Enter')
  await expect(page.getByTestId('text-kerning-value')).toBeEnabled()
}

async function setValue(page: Page, scope: string, value: number) {
  await page.getByTestId('text-kerning-scope').selectOption(scope)
  const input = page.getByTestId('text-kerning-value')
  await expect(input).toBeEnabled()
  await input.fill(String(value))
  await input.press('Enter')
}

test('authors groups and pair exceptions, coalesces shortcuts, saves and exports matching layout', async ({
  page,
  source,
}) => {
  await seedGroups(page, source.doc)
  const canvas = await enterText(page)
  await selectFirstGap(page, source.doc.units_per_em)

  for (const [scope, value, advance] of [
    ['group_group', -100, 1100],
    ['group_glyph', -40, 1160],
    ['glyph_group', -60, 1140],
    ['glyph_glyph', 0, 1200],
  ] as const) {
    await setValue(page, scope, value)
    await expect
      .poll(async () => (await resolution(page, source.doc)).value)
      .toBe(value)
    await expectAdvance(page, source.doc, advance)
    await expectPreviewAdvance(page, advance)
  }
  await page.screenshot({ path: test.info().outputPath('text-kerning.png') })
  await page.getByTestId('text-kerning-remove').click()
  await expect
    .poll(async () => (await resolution(page, source.doc)).value)
    .toBe(-60)
  await canvas.press('Control+z')
  await expect
    .poll(async () => (await resolution(page, source.doc)).value)
    .toBe(0)

  await setValue(page, 'glyph_glyph', -80)
  await expectPreviewAdvance(page, 1120)
  await page.getByRole('button', { name: 'kern', exact: true }).click()
  await expectPreviewAdvance(page, 1200)
  await page.getByRole('button', { name: 'kern', exact: true }).click()
  await expectPreviewAdvance(page, 1120)
  const value = page.getByTestId('text-kerning-value')
  await value.fill('-123')
  await value.press('Escape')
  await expect(value).toHaveValue('-80')
  await canvas.press('Control+Alt+ArrowRight')
  await canvas.press('Control+Alt+ArrowRight')
  await canvas.press('Control+Alt+ArrowRight')
  await expect
    .poll(async () => (await resolution(page, source.doc)).value)
    .toBe(-77)
  await canvas.press('Control+z')
  await expect
    .poll(async () => (await resolution(page, source.doc)).value)
    .toBe(-80)

  const native = join(source.directory, 'Authored.fcf')
  const ttf = join(source.directory, 'Authored.ttf')
  await saveAs(page, native)
  await exportThroughDialog(page, ttf)
  expect(existsSync(native)).toBe(true)
  expect(existsSync(ttf)).toBe(true)
  const reopened = await openSource(page, native)
  await expectAdvance(page, reopened, 1120)
  expect((await resolution(page, reopened)).records).toHaveLength(4)
  const binary = await openSource(page, ttf)
  await expectAdvance(page, binary, 1120)
  await expectAdvance(page, binary, 1200, false)
})

test('creates, assigns, renames and explicitly removes an in-use shared group', async ({
  page,
  source,
}) => {
  await openGlyphFromGrid(page, 'A')
  const first = page.getByRole('group', { name: 'First side', exact: true })
  await first.getByRole('button', { name: 'Manage First side groups' }).click()
  await first
    .getByRole('textbox', { name: 'New First side group name' })
    .fill('A')
  await first.getByRole('button', { name: 'Create and assign' }).click()
  await expect
    .poll(
      async () =>
        (
          await api<GlyphKerning>(page, 'kerning.glyph', {
            doc: source.doc.doc,
            glyph: 'A',
          })
        ).side1,
    )
    .toBe('public.kern1.A')

  await page
    .getByRole('listbox', { name: 'Glyph list' })
    .locator('[data-glyph="a"]')
    .click()
  await first.getByRole('button', { name: 'Manage First side groups' }).click()
  await first.getByRole('button', { name: 'Inspect public.kern1.A' }).click()
  await first.getByRole('button', { name: 'Assign a', exact: true }).click()
  await expect
    .poll(
      async () =>
        (
          await api<GlyphKerning>(page, 'kerning.glyph', {
            doc: source.doc.doc,
            glyph: 'a',
          })
        ).side1,
    )
    .toBe('public.kern1.A')
  await api(page, 'doc.apply', {
    doc: source.doc.doc,
    label: 'Prepare in-use group',
    command: {
      SetHorizontalKerning: {
        master: source.doc.masters[0].id,
        left: { group: 'public.kern1.A' },
        right: { glyph: 'V' },
        value: -80,
      },
    },
  })

  await first.getByRole('button', { name: 'Manage First side groups' }).click()
  await first.getByRole('button', { name: 'Inspect public.kern1.A' }).click()
  await first
    .getByRole('textbox', { name: 'Rename public.kern1.A' })
    .fill('Diagonal')
  await first.getByRole('button', { name: 'Rename group', exact: true }).click()
  await expect
    .poll(async () => (await resolution(page, source.doc)).left_group)
    .toBe('public.kern1.Diagonal')
  await expectAdvance(page, source.doc, 1120)
  await first.getByRole('button', { name: 'Manage First side groups' }).click()
  await first
    .getByRole('button', { name: 'Inspect public.kern1.Diagonal' })
    .click()
  await first
    .getByRole('button', { name: 'Remove group…', exact: true })
    .click()
  await expect(
    first.getByText(/Horizontal pair records removed across all masters: 1\./),
  ).toBeVisible()
  await page.screenshot({ path: test.info().outputPath('group-impact.png') })
  await first
    .getByRole('button', { name: 'Remove group and pairs', exact: true })
    .click()
  await expect
    .poll(async () => (await resolution(page, source.doc)).value)
    .toBe(0)
  await page.getByRole('img', { name: /^Glyph / }).press('Control+z')
  await expect
    .poll(async () => (await resolution(page, source.doc)).left_group)
    .toBe('public.kern1.Diagonal')
  await expectAdvance(page, source.doc, 1120)
})

test('settles a delayed pair edit before master handover and ignores its late selection response', async ({
  page,
  source,
}) => {
  await enterText(page)
  await choosePair(page)
  const held = await holdNextKerning(page)
  try {
    await setValue(page, 'glyph_glyph', -80)
    await held.wait()
    await expect
      .poll(async () => (await resolution(page, source.doc)).value)
      .toBe(0)
    await page
      .getByRole('toolbar', { name: 'Text controls' })
      .getByRole('combobox', { name: 'Master', exact: true })
      .selectOption(source.doc.masters[1].id)
    held.release()
    await expect
      .poll(async () => (await resolution(page, source.doc)).value)
      .toBe(-80)
    await expect(
      page
        .getByRole('toolbar', { name: 'Text controls' })
        .getByRole('combobox', { name: 'Master', exact: true }),
    ).toHaveValue(source.doc.masters[1].id)
    await choosePair(page)
    await expect(page.getByTestId('text-kerning-value')).toHaveValue('0')
    await setValue(page, 'glyph_glyph', -120)
    await expect
      .poll(
        async () =>
          (await resolution(page, source.doc, source.doc.masters[1].id)).value,
      )
      .toBe(-120)
    await expectAdvance(page, source.doc, 1120)
    await expectAdvance(page, source.doc, 1080, true, source.doc.masters[1].id)
  } finally {
    await held.dispose()
  }
})

test('keeps a failed delayed edit out of stored kerning and allows recovery', async ({
  page,
  source,
}) => {
  await enterText(page)
  await choosePair(page)
  await setValue(page, 'glyph_glyph', -80)
  await expectAdvance(page, source.doc, 1120)
  const held = await holdNextKerning(page, true)
  try {
    await setValue(page, 'glyph_glyph', -90)
    await held.wait()
    held.release()
    await expect(page.getByText(/P2a delayed kerning failure/)).toBeVisible()
    await expect
      .poll(async () => (await resolution(page, source.doc)).value)
      .toBe(-80)
    await expect(page.getByTestId('text-kerning-value')).toHaveValue('-80')
  } finally {
    await held.dispose()
  }
  await setValue(page, 'glyph_glyph', -70)
  await expectAdvance(page, source.doc, 1130)
})

test('saves a pending edit and keeps document handover isolated', async ({
  page,
  source,
}) => {
  await enterText(page)
  await choosePair(page)
  const native = join(source.directory, 'Pending.fcf')
  const held = await holdNextKerning(page)
  let saveRequested = false
  page.on('request', (request) => {
    if (request.url().endsWith('/api/session.save')) saveRequested = true
  })
  try {
    await setValue(page, 'glyph_glyph', -80)
    await held.wait()
    await menuItem(page, 'File', /^Save As/)
    const dialog = page.getByRole('dialog', { name: 'Save As' })
    await dialog.getByRole('textbox', { name: 'Document path' }).fill(native)
    await dialog.getByRole('button', { name: 'Save', exact: true }).click()
    expect(saveRequested).toBe(false)
    expect(existsSync(native)).toBe(false)
    held.release()
    await expect(dialog).toBeHidden()
    await expect.poll(() => saveRequested && existsSync(native)).toBe(true)
  } finally {
    await held.dispose()
  }
  const reopened = await openSource(page, native)
  await expectAdvance(page, reopened, 1120)
  const secondFiles = sourceFixture()
  try {
    const second = await openSource(page, secondFiles.input)
    // Reopening a saved document restores its own text tab, so more than one
    // "AV" tab can exist; this test means the one of the edited document.
    await page
      .locator(`[role="tab"][data-tab^="glyph:${source.doc.doc}:"]`)
      .first()
      .click()
    await choosePair(page)
    const switching = await holdNextKerning(page)
    try {
      await setValue(page, 'glyph_glyph', -90)
      await switching.wait()
      // An unlabelled Overview tab renders as `Overview · <file>`, so match the
      // file name at the start of the tab name or after that separator.
      await page.getByRole('tab', { name: /(?:^|· )Regular\.ufo$/ }).click()
      switching.release()
      await expect
        .poll(async () => (await resolution(page, source.doc)).value)
        .toBe(-90)
      await enterText(page)
      await choosePair(page)
      await expect(page.getByTestId('text-kerning-value')).toHaveValue('0')
      expect((await resolution(page, second)).value).toBe(0)
    } finally {
      await switching.dispose()
    }
  } finally {
    secondFiles.cleanup()
  }
})

test('keeps imported spacing previewable and explains unavailable authoring', async ({
  page,
  source,
}) => {
  expect(source.doc.masters).toHaveLength(2)
  const imported = await openSource(page, fixture('DejaVuSansMin.woff2'))
  const status = await api<{ editable: boolean; reason: string }>(
    page,
    'kerning.status',
    { doc: imported.doc },
  )
  expect(status).toMatchObject({ editable: false, reason: 'imported_layout' })
  await enterText(page)
  await expect(page.getByTestId('text-kerning-panel')).toContainText(
    /spacing rules cannot yet be edited/,
  )
  await expect(page.getByTestId('text-kerning-value')).toBeDisabled()
  await menuItem(page, 'File', /^Font Info/)
  await expect(
    page.getByRole('tab', { name: 'Font Info', exact: true }),
  ).toBeVisible()
})

/** The stored metrics of one glyph in one master, read through the spacing API. */
async function layerMetrics(
  page: Page,
  doc: DocSummary,
  name: string,
  master: string,
) {
  const answer = await api<{
    rows: {
      width: number
      lsb: number | null
      rsb: number | null
      out_of_sync: boolean
      beam_left: number | null
    }[]
  }>(page, 'metrics.spacing', {
    doc: doc.doc,
    master,
    glyphs: [name],
    beam: null,
  })
  const row = answer.rows[0]
  if (!row) throw new Error(`no spacing row for ${name}`)
  return row
}

test('edits metrics formulas with inline validation and synchronises every master', async ({
  page,
  source,
}) => {
  const [regular, bold] = source.doc.masters
  await openGlyphFromGrid(page, 'A')
  const field = page.getByRole('textbox', { name: 'LSB key' })
  await expect(field).toBeVisible()

  // A malformed formula is refused in place; nothing reaches the document.
  await field.fill('=V+')
  await field.press('Enter')
  await expect(page.getByRole('alert')).toContainText(
    'needs a number after the operator',
  )
  await expect(field).toHaveAttribute('aria-invalid', 'true')
  const before = await layerMetrics(page, source.doc, 'A', regular!.id)
  expect(before.lsb ?? 0).toBeGreaterThan(0)

  // The bare spelling of a reference is accepted as the formula it stands for.
  await field.fill('V')
  await field.press('Enter')
  await expect(field).toHaveValue('=V')
  // Arithmetic on the reference, so the stored value cannot already agree.
  await field.fill('=V+25')
  await field.press('Enter')
  await expect(field).toHaveValue('=V+25')
  await expect(page.getByText('out of sync').first()).toBeVisible()

  // Synchronising through the Glyph menu covers every master in one entry.
  await menuItem(page, 'Glyph', /^Update Metrics in All Masters/)
  await expect(page.getByText('In sync').first()).toBeVisible()
  for (const master of [regular!, bold!]) {
    const after = await layerMetrics(page, source.doc, 'A', master.id)
    const reference = await layerMetrics(page, source.doc, 'V', master.id)
    expect(Math.round(after.lsb ?? 0)).toBe(
      Math.round((reference.lsb ?? 0) + 25),
    )
    expect(after.out_of_sync).toBe(false)
  }
  await menuItem(page, 'Edit', /^Undo/)
  await expect
    .poll(
      async () => (await layerMetrics(page, source.doc, 'A', regular!.id)).lsb,
    )
    .toBe(before.lsb)
})

test('measures and edits the run through the spacing matrix, beam and cut-off', async ({
  page,
  source,
}) => {
  const master = source.doc.masters[0]!.id
  await enterText(page, 'AV')
  await page.getByText('Spacing matrix').click()
  const cell = page.getByRole('textbox', { name: 'LSB of A' })
  await expect(cell).toBeVisible()
  const before = await layerMetrics(page, source.doc, 'A', master)

  await cell.fill(String(Math.round(before.lsb ?? 0) + 30))
  await cell.press('Enter')
  await expect
    .poll(async () => (await layerMetrics(page, source.doc, 'A', master)).width)
    .toBe(before.width + 30)

  // A formula typed into the same cell replaces the number.
  await cell.fill('=V')
  await cell.press('Enter')
  await expect(
    page.getByRole('button', { name: 'Update' }).first(),
  ).toBeVisible()
  await page.getByRole('button', { name: 'Update' }).first().click()
  await expect(cell).toHaveValue('=V')

  // The beam reports each glyph's margins at its height, and the run redraws.
  await page.getByRole('checkbox', { name: 'Beam' }).check()
  await expect(
    page.getByRole('columnheader', { name: 'Beam LSB' }),
  ).toBeVisible()
  const beamCells = page
    .locator('tr[data-glyph="A"] td')
    .filter({ hasText: /\d/ })
  await expect
    .poll(async () => (await beamCells.allTextContents()).join(' '))
    .toMatch(/\d/)
  await page.getByRole('slider', { name: 'Drag the beam height' }).fill('120')
  await expect(
    page.getByRole('spinbutton', { name: 'Beam height' }),
  ).toHaveValue('120')

  // The beam is also dragged on the canvas itself.
  const canvas = page.getByRole('img', { name: /^Glyph / })
  await page.getByRole('button', { name: 'Fit Glyph in Window' }).click()
  const at = async (height: number) =>
    canvas.evaluate((element, font) => {
      const globals = globalThis as unknown as {
        __fwTextRun?: { activeOrigin: { x: number; y: number } }
        __fwEditor?: {
          viewportForTest: {
            toScreen: (point: { x: number; y: number }) => {
              x: number
              y: number
            }
          }
        }
      }
      const run = globals.__fwTextRun
      const editor = globals.__fwEditor
      if (!run || !editor) throw new Error('the text run is not ready')
      const point = editor.viewportForTest.toScreen({
        x: 0,
        y: font - run.activeOrigin.y,
      })
      const bounds = (
        element as unknown as {
          getBoundingClientRect: () => {
            left: number
            top: number
            width: number
          }
        }
      ).getBoundingClientRect()
      // Well away from the gap handle, which is offered before the beam.
      return { x: bounds.left + bounds.width * 0.15, y: bounds.top + point.y }
    }, height)
  const from = await at(120)
  const to = await at(420)
  await page.mouse.move(from.x, from.y)
  await page.mouse.down()
  await page.mouse.move(to.x, to.y, { steps: 4 })
  await page.mouse.up()
  await expect
    .poll(async () =>
      Number(
        await page
          .getByRole('spinbutton', { name: 'Beam height' })
          .inputValue(),
      ),
    )
    .toBeGreaterThan(380)

  await page.getByRole('checkbox', { name: 'Cut off' }).check()
  await expect(
    page.getByRole('spinbutton', { name: 'Cut off from' }),
  ).toBeVisible()
  await page.screenshot({
    path: test.info().outputPath('spacing-matrix.png'),
  })
  // The canvas alone, so the beam line, its measurement and the cut-off band
  // are legible at the size the acceptance report embeds.
  await page.getByRole('button', { name: 'Fit Glyph in Window' }).click()
  await canvas.screenshot({
    path: test.info().outputPath('beam-cutoff.png'),
  })
})

test('marks kerning exceptions and maintains the table across masters', async ({
  page,
  source,
}) => {
  const [regular, bold] = source.doc.masters
  await seedGroups(page, source.doc)
  for (const [left, right, value] of [
    [{ group: 'public.kern1.A' }, { group: 'public.kern2.V' }, -60],
    [{ glyph: 'A' }, { group: 'public.kern2.V' }, -20],
  ] as const) {
    await api(page, 'doc.apply', {
      doc: source.doc.doc,
      label: 'Seed kerning',
      command: {
        SetHorizontalKerning: { master: regular!.id, left, right, value },
      },
    })
  }
  await enterText(page, 'AV')
  await page.getByText('Kerning table').click()
  const table = page.getByRole('table')
  await expect(table).toBeVisible()
  await expect(table.getByText('Exception')).toBeVisible()
  await expect(table.getByText('Group')).toBeVisible()

  await page
    .getByRole('combobox', { name: 'Record kind' })
    .selectOption('exceptions')
  await expect(table.getByText('public.kern1.A')).toBeHidden()
  await page.getByRole('combobox', { name: 'Record kind' }).selectOption('all')

  // Copy this master's table, switch the table's master and keep existing.
  await page.getByRole('button', { name: 'Copy table' }).click()
  await page
    .getByRole('combobox', { name: 'Table master' })
    .selectOption(bold!.id)
  await page.getByRole('combobox', { name: 'On conflict' }).selectOption('keep')
  await page.getByRole('button', { name: 'Paste table' }).click()
  await expect
    .poll(async () => (await resolution(page, source.doc, bold!.id)).value)
    .toBe(-20)

  // Compress folds the agreeing pairs of that master back into one group rule.
  await page.getByRole('button', { name: 'Compress' }).click()
  await expect
    .poll(async () => (await resolution(page, source.doc, bold!.id)).value)
    .toBe(-20)
  await expect
    .poll(async () => (await resolution(page, source.doc, bold!.id)).exception)
    .toBe(true)
  await page.screenshot({
    path: test.info().outputPath('kerning-table.png'),
  })
})

test('locks every spacing control while kerning with spacing locked', async ({
  page,
  source,
}) => {
  const master = source.doc.masters[0]!.id
  const canvas = await enterText(page, 'AV')
  const before = await layerMetrics(page, source.doc, 'A', master)
  await page
    .getByRole('combobox', { name: 'Kerning mode' })
    .selectOption('locked')
  await expect(
    page.getByRole('combobox', { name: 'Spacing side' }),
  ).toBeDisabled()
  // The matrix is the tab's own spacing surface; the inspector's fields are
  // covered by MetricsPanel and InfoBox unit tests, which render in edit mode.
  await page.getByText('Spacing matrix').click()
  await expect(page.getByRole('textbox', { name: 'LSB of A' })).toBeDisabled()
  await canvas.press(']')
  await canvas.press(']')
  await expect
    .poll(async () => (await layerMetrics(page, source.doc, 'A', master)).width)
    .toBe(before.width)
})

test('edits vertical kerning and vertical metrics in the vertical direction', async ({
  page,
  source,
}) => {
  const master = source.doc.masters[0]!.id
  await enterText(page, 'AV')
  await page.getByRole('button', { name: 'Vertical text', exact: true }).click()
  await api(page, 'doc.apply', {
    doc: source.doc.doc,
    label: 'Seed vertical kerning',
    command: {
      SetVerticalKerning: {
        master,
        left: { glyph: 'A' },
        right: { glyph: 'V' },
        value: -35,
      },
    },
  })
  await expect
    .poll(
      async () =>
        (
          await api<{ value: number | null }>(
            page,
            'kerning.resolve_vertical',
            {
              doc: source.doc.doc,
              master,
              left: 'A',
              right: 'V',
            },
          )
        ).value,
    )
    .toBe(-35)
  await page.getByText('Kerning table').click()
  await expect(page.getByRole('table').getByText('-35')).toBeVisible()
})
