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
    first.getByText(
      /This also removes 1 horizontal pair records? across all masters\./,
    ),
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
    await page.getByRole('tab', { name: 'AV', exact: true }).click()
    await choosePair(page)
    const switching = await holdNextKerning(page)
    try {
      await setValue(page, 'glyph_glyph', -90)
      await switching.wait()
      await page.getByRole('tab', { name: 'Regular.ufo', exact: true }).click()
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
